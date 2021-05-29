use mysql_async::Pool;
use mysql_async_support_model::Error;

/// Trait for an async function with a restricted lifetime scope.
pub trait FnWithPool<'pool> {
    type Output;
    type Error: From<Error>;
    type Fut: std::future::Future<Output = (Pool, Result<Self::Output, Self::Error>)> + 'pool;

    /// Runs the function
    fn call(self, pool: Pool) -> Self::Fut;
}

impl<'pool, F, Fut, T, E> FnWithPool<'pool> for F
where
    F: FnOnce(Pool) -> Fut,
    E: From<Error>,
    Fut: std::future::Future<Output = (Pool, Result<T, E>)> + 'pool,
{
    type Error = E;
    type Fut = Fut;
    type Output = T;

    fn call(self, pool: Pool) -> Fut {
        self(pool)
    }
}
