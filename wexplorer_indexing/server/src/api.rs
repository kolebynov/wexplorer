
use tonic::{Request, Response, Status};

use crate::indexing::{Indexer, UrlProcessor};

use self::indexing_api_server::IndexingApi;

tonic::include_proto! {"indexing"}

pub struct IndexingApiImpl<U> {
    pub indexer: Indexer<U>,
}

#[tonic::async_trait]
impl<U> IndexingApi for IndexingApiImpl<U>
where
    U: UrlProcessor + Clone + Send + Sync + 'static,
{
    async fn index_web_site(&self, request: Request<IndexWebSiteRequest>) -> Result<Response<()>, Status> {
        self.indexer.index_page(request.get_ref().origin.parse().map_err(|_| Status::invalid_argument("origin"))?).await;
        Ok(Response::new(()))
    }

    async fn get_indexing_web_sites(&self, _: Request<()>) -> Result<Response<GetIndexingWebSitesResponse>, Status> {
        Ok(Response::new(GetIndexingWebSitesResponse {
            origins: vec![],
        }))
    }

    async fn get_indexing_pages(&self, _: Request<()>) -> Result<Response<GetIndexingPagesResponse>, Status> {
        Ok(Response::new(GetIndexingPagesResponse {
            pages: vec![],
        }))
    }
}