#![feature(round_char_boundary)]

use api::{searching_api_server::SearchingApiServer, SearchingApiImpl};
use tonic::transport::Server;
use tracing::Level;

mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt().with_thread_ids(true).with_target(false).with_max_level(Level::INFO).finish()
    ).unwrap();

    Server::builder()
        .add_service(SearchingApiServer::new(SearchingApiImpl::new()))
        .serve("0.0.0.0:8083".parse()?)
        .await?;

    Ok(())
}