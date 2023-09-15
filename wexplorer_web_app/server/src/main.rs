use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{CorsLayer, Any};
use web_searching_api::{web_searching_api_server::WebSearchingApiServer, WebSearchingApiImpl};

mod web_searching_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::builder()
        .accept_http1(true)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .layer(GrpcWebLayer::new())
        .add_service(WebSearchingApiServer::new(WebSearchingApiImpl::new()))
        .serve("0.0.0.0:8081".parse()?)
        .await?;

    Ok(())
}
