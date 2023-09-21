use api::{searching_api_server::SearchingApiServer, SearchingApiImpl};
use app_infrastructure::{app_config::AppConfigurationBuilder, app_tracing, BoxError, tonic::ConfigurableServer};

mod api;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let app_config = AppConfigurationBuilder::default().build()?;
    app_tracing::init_from_config(&app_config.config)?;

    ConfigurableServer::builder(&app_config.config)
        .add_service(SearchingApiServer::new(SearchingApiImpl::new()))
        .serve()
        .await?;

    Ok(())
}