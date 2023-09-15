use tokio::sync::Mutex;
use tonic::{transport::{Channel, Endpoint}, Request, Response, Status, IntoRequest};
use wexplorer_searching_grpc_client::{searching_api_client::SearchingApiClient, SearchRequest, SearchResponse, search_response};

use self::web_searching_api_server::WebSearchingApi;

tonic::include_proto! {"web_searching"}

pub struct WebSearchingApiImpl {
    searching_grpc_client: Mutex<SearchingApiClient<Channel>>,
}

impl WebSearchingApiImpl {
    pub fn new() -> Self {
        Self {
            searching_grpc_client: Mutex::new(
                SearchingApiClient::new(Endpoint::from_static("http://localhost:8083").connect_lazy())),
        }
    }
}

#[tonic::async_trait]
impl WebSearchingApi for WebSearchingApiImpl {
    async fn search(&self, request: Request<WebSearchRequest>) -> Result<Response<WebSearchResponse>, Status> {
        self.searching_grpc_client.lock().await
            .search(request.into_inner()).await
            .map(|r| Response::new(r.into_inner().into()))
    }
}

impl IntoRequest<SearchRequest> for WebSearchRequest {
    fn into_request(self) -> Request<SearchRequest> {
        Request::new(SearchRequest { text: self.text })
    }
}

impl From<SearchResponse> for WebSearchResponse {
    fn from(value: SearchResponse) -> Self {
        Self {
            results: value.results.into_iter()
                .map(|r| web_search_response::Result {
                    url: r.url,
                    entries: r.entries.into_iter()
                        .map(|e| web_search_response::result::FoundEntry { text: e.text })
                        .collect(),
                })
                .collect(),
        }
    }
}