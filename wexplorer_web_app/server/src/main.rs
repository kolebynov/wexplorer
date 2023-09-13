use api::{echo_service_server::{EchoService, EchoServiceServer}, EchoRequest, EchoResponse};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{CorsLayer, Any};

mod api {
    tonic::include_proto! {"api"}
}

#[derive(Debug, Default)]
pub struct Echo {}

#[tonic::async_trait]
impl EchoService for Echo {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        let reply = EchoResponse {
            message: format!("{}", request.into_inner().message),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::builder()
        .accept_http1(true)
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any))
        .layer(GrpcWebLayer::new())
        .add_service(EchoServiceServer::new(Echo {}))
        .serve("0.0.0.0:8081".parse()?)
        .await?;

    Ok(())
}
