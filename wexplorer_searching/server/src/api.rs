use std::sync::Mutex;

use num::clamp;
use tonic::{Request, Response, Status};
use tracing::info;

use self::{searching_api_server::SearchingApi, search_response::result::FoundEntry};

tonic::include_proto! {"searching"}

struct PageData {
    url: String,
    text: String,
}

pub struct SearchingApiImpl {
    storage: Mutex<Vec<PageData>>,
}

impl SearchingApiImpl {
    pub fn new() -> Self {
        Self { storage: Mutex::new(vec![]) }
    }
}

#[tonic::async_trait]
impl SearchingApi for SearchingApiImpl {
    async fn add_page(&self, request: Request<AddPageRequest>) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let mut storage = self.storage.lock().unwrap();
        if let Some(page_data) = storage.iter_mut().find(|p| p.url == request.url) {
            info!("Updating text for url {}", request.url);
            page_data.text = request.text;
        }
        else {
            info!("Adding page to the storage {}", request.url);
            storage.push(PageData { url: request.url, text: request.text });
        }

        Ok(Response::new(()))
    }

    async fn search(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        let request = request.into_inner();
        let results = self.storage.lock().unwrap()
            .iter()
            .filter_map(|p| {
                let mut entries = vec![];
                let mut index: i32 = 0;
                loop {
                    let slice = &p.text[index as usize..];
                    if let Some(found_index) = slice.find(&request.text) {
                        index += found_index as i32;
                        const OFFSET: i32 = 30;
                        let text_start = p.text.floor_char_boundary(clamp(index - OFFSET, 0, p.text.len() as i32) as usize) as i32;
                        let text_end = p.text.ceil_char_boundary(clamp(index as usize + request.text.len() + OFFSET as usize, 0, p.text.len()));
                        entries.push(FoundEntry {
                            text: p.text[text_start as usize..text_end].to_string(),
                            highlight_start: index - text_start,
                            highlight_end: index - text_start + request.text.len() as i32,
                        });
                        index += request.text.len() as i32;
                    }
                    else {
                        break;
                    }
                }

                if entries.is_empty() { None } else { Some(search_response::Result { url: p.url.clone(), entries }) }
            })
            .collect();

        Ok(Response::new(SearchResponse { results }))
    }
}