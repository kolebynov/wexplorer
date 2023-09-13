use std::sync::Mutex;

use num::clamp;
use opensearch::{OpenSearch, http::transport::Transport, IndexParts};
use serde_json::json;
use tonic::{Request, Response, Status};
use tracing::info;

use self::{searching_api_server::SearchingApi, search_response::result::FoundEntry};

tonic::include_proto! {"searching"}

pub struct SearchingApiImpl {
    open_search_client: OpenSearch,
}

impl SearchingApiImpl {
    pub fn new() -> Self {
        Self { open_search_client: OpenSearch::new(Transport::single_node("http://127.0.0.1:9200").unwrap()) }
    }
}

#[tonic::async_trait]
impl SearchingApi for SearchingApiImpl {
    async fn add_page(&self, request: Request<AddPageRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let body = json!({
            "url": request.url,
            "text": request.text,
        });
        self.open_search_client
            .index(IndexParts::Index("search_index"))
            .body(body)
            .send()
            .await
            .map_err(|err| {
                Status::internal(err.to_string())
            })?;
        Ok(Response::new(()))
    }

    async fn search(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        Ok(Response::new(SearchResponse { ..Default::default() }))
    }
}