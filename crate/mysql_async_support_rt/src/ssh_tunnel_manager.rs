use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use futures::{stream, StreamExt, TryStreamExt};
use mysql_async_support_model::{Error, QueryTarget, SshTunnelMap};
use ssh_jumper::{
    model::{HostAddress, HostSocketParams, JumpHostAuthParams},
    SshJumper,
};

/// Opens SSH sessions and creates tunnels for query targets.
#[derive(Debug)]
pub struct SshTunnelManager;

impl SshTunnelManager {
    /// When we use `0` as the local port to forward, the OS will choose a free
    /// port.
    const LOCAL_OS_CHOSEN_PORT: u16 = 0;

    /// Opens an SSH session and creates a tunnel per query target.
    ///
    /// The [`SshSession`] returned by this function should be kept alive until
    /// all the tunnels are no longer needed.
    ///
    /// Callers of this function should appropriately limit the number of query
    /// targets per tunnel, perhaps by calling [`chunks`].
    ///
    /// [`chunks`]: std::slice::chunks
    pub async fn prepare_tunnels<'qt>(
        jump_host_addr: &HostAddress<'_>,
        jump_host_auth_params: &JumpHostAuthParams<'_>,
        query_targets: &'qt [QueryTarget<'qt>],
    ) -> Result<SshTunnelMap<'qt>, Error> {
        let ssh_session =
            SshJumper::open_ssh_session(jump_host_addr, jump_host_auth_params).await?;
        let ssh_session_ref = &ssh_session;

        let qt_name_to_tunnel = stream::iter(query_targets)
            .map(Result::<_, Error>::Ok)
            .try_fold(
                HashMap::with_capacity(query_targets.len()),
                |mut qt_name_to_tunnel, query_target| async move {
                    let local_socket = SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        Self::LOCAL_OS_CHOSEN_PORT,
                    );

                    let target_socket = HostSocketParams {
                        address: query_target.db_address.clone(),
                        port: 3306,
                    };
                    let ssh_tunnel = SshJumper::open_direct_channel(
                        ssh_session_ref,
                        local_socket,
                        &target_socket,
                    )
                    .await?;

                    qt_name_to_tunnel.insert(query_target.name.as_ref(), ssh_tunnel);

                    Ok(qt_name_to_tunnel)
                },
            )
            .await?;

        let ssh_tunnel_map = SshTunnelMap {
            ssh_session,
            qt_name_to_tunnel,
        };

        Ok(ssh_tunnel_map)
    }
}
