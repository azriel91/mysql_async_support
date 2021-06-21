use std::{
    collections::HashMap,
    fmt,
    net::SocketAddr,
    ops::{Deref, DerefMut},
};

use ssh_jumper::SshSession;

/// Keeps SshSession alive while tunnels are needed.
///
/// This is needed to keep the SSH session alive, because the lifetime of the
/// [`SocketAddr`] is linked to the [`SshSession`].
pub struct SshTunnelMap<'qt> {
    /// The SSH session the tunnels are created from.
    pub ssh_session: SshSession,
    /// Mapping between query target name and socket address.
    pub qt_name_to_tunnel: HashMap<&'qt str, SocketAddr>,
}

impl<'qt> Deref for SshTunnelMap<'qt> {
    type Target = HashMap<&'qt str, SocketAddr>;

    fn deref(&self) -> &Self::Target {
        &self.qt_name_to_tunnel
    }
}

impl<'qt> DerefMut for SshTunnelMap<'qt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.qt_name_to_tunnel
    }
}

impl<'qt> fmt::Debug for SshTunnelMap<'qt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // AsyncSession which backs `SshSession` is `!Debug`.
        // https://docs.rs/async-ssh2-lite/latest/async_ssh2_lite/struct.AsyncSession.html
        f.debug_struct("SshTunnelMap")
            .field("qt_name_to_tunnel", &self.qt_name_to_tunnel)
            .finish()
    }
}
