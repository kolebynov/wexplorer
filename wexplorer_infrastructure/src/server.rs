use std::error::Error;

use config::Config;
use tonic::async_trait;

#[async_trait]
pub trait ConfigurableServerBuilder: Sized {
    type Error: Error;

    async fn server_with_config(self, config: &Config) -> Result<(), Self::Error>;
}