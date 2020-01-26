use crate::connection::Connection;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod ssl;
pub mod tcp;

#[async_trait]
pub trait Stream: Send + Sync
where
    Self::Out: AsyncWrite + AsyncRead,
{
    type Out;
    async fn accept(&mut self) -> Result<Connection<Self::Out>, Box<dyn std::error::Error>>;
}
