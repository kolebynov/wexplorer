use std::{sync::{Arc, atomic::AtomicU64}, task::{Context, Poll}, fmt::Debug};

use api::{IndexingApiImpl, indexing_api_server::IndexingApiServer};
use indexing::{Indexer, AllowedSchemeUrlFilter, UrlNormalizerBuilder, RemoveFragmentNormalizer, UrlProcessorImpl, UrlProcessor, RemoveQueryParamsNormalizer, RemoveQueryParam, QueryParamMatchType, SortQueryParamsNormalizer, SchemeToLowerCaseNormalizer, TextExtractor};
use queue::IndexingQueue;
use tonic::transport::Server;
use tower::{Layer, Service};
use tracing::{Instrument, instrument::Instrumented, error_span, Level};
use tracing_log::LogTracer;
use url::Url;

mod api;
mod queue;
mod indexing;

#[derive(Clone)]
struct LogLayer;

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, service: S) -> Self::Service {
        LogService {
            service,
            counter: Arc::new(0.into()),
        }
    }
}

#[derive(Clone)]
struct LogService<S> {
    service: S,
    counter: Arc<AtomicU64>,
}

impl<S, Request> Service<Request> for LogService<S>
where
    S: Service<Request>,
    Request: Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Instrumented<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.service.call(request).instrument(error_span!("request", number = self.counter.load(std::sync::atomic::Ordering::SeqCst)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt().with_thread_ids(true).with_target(false).with_max_level(Level::INFO).finish()
    ).unwrap();
    LogTracer::init().unwrap();

    let url_filter = AllowedSchemeUrlFilter::new(vec!["http".to_string(), "https".to_string()]);
    let url_normalizer = UrlNormalizerBuilder::new()
        .add_normalizer(RemoveFragmentNormalizer {})
        .add_normalizer(RemoveQueryParamsNormalizer::new(vec![RemoveQueryParam(QueryParamMatchType::StartWith, "utm_".to_string())]))
        .add_normalizer(SortQueryParamsNormalizer {})
        .add_normalizer(SchemeToLowerCaseNormalizer {})
        .build();
    let mut indexer = Indexer::new(
        IndexingQueue::new(), UrlProcessorImpl::new(url_filter, url_normalizer), TextExtractor::new());
    indexer.start_processing(1);
    Server::builder()
        .layer(LogLayer {})
        .add_service(IndexingApiServer::new(IndexingApiImpl { indexer }))
        .serve("0.0.0.0:8082".parse()?)
        .await?;

    Ok(())
}