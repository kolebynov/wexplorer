use std::{sync::Arc, future::Future, pin::Pin, task::{Context, Poll}, time::Duration, rc::Rc};

use itertools::Itertools;
use reqwest::redirect::Policy;
use scraper::{Selector, Element};
use tokio::{task::{JoinHandle, futures}, select, sync::oneshot};
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};
use tracing::{info, Instrument, trace_span, info_span, span, Level, error_span, warn};
use url::Url;
use wexplorer_searching_grpc_client::{searching_api_client::SearchingApiClient, AddPageRequest};

use crate::queue::IndexingQueue;

use super::{url_processing::{UrlProcessor, UrlProcessorImpl, AllowedSchemeUrlFilter, UrlNormalizerBuilder, RemoveFragmentNormalizer}, text_extracting::TextExtractor};

struct WithCancellation<'a, T> {
    inner: Pin<Box<T>>,
    cancellation_future: Pin<Box<WaitForCancellationFuture<'a>>>,
}

impl<'a, T: Future> Future for WithCancellation<'a, T> {
    type Output = Option<T::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(res) => Poll::Ready(Some(res)),
            Poll::Pending => self.cancellation_future.as_mut().poll(cx).map(|_| None),
        }
    }
}

trait FutureWithCancellation: Future + Sized {
    fn with_cancellation(self, token: &CancellationToken) -> WithCancellation<'_, Self> {
        WithCancellation { inner: Box::pin(self), cancellation_future: Box::pin(token.cancelled()) }
    }
}

impl<T: Future + Sized> FutureWithCancellation for T {}

trait SendSyncUrlProcessor: UrlProcessor + Send + Sync {}

impl<T: UrlProcessor + Send + Sync> SendSyncUrlProcessor for T {}

pub struct Indexer<U> {
    queue: Arc<IndexingQueue>,
    processing_handles: Vec<JoinHandle<()>>,
    cancellation_token: CancellationToken,
    url_processor: U,
    text_extractor: TextExtractor,
}

impl<U> Indexer<U>
where
    U: UrlProcessor + Clone + Send + 'static
{
    pub fn new(queue: IndexingQueue, url_processor: U, text_extractor: TextExtractor) -> Self {
        Self {
            queue: Arc::new(queue),
            processing_handles: Vec::new(),
            cancellation_token: CancellationToken::new(),
            url_processor,
            text_extractor,
        }
    }

    pub async fn index_page(&self, url: Url) {
        if let Some(url) = self.url_processor.process_url(url) {
            self.queue.enqueue(url).await;
        }
    }

    pub fn start_processing(&mut self, worker_count: u32) {
        self.processing_handles.clear();

        for i in 0..worker_count {
            let queue = self.queue.clone();
            let ct = self.cancellation_token.clone();
            let url_processor = self.url_processor.clone();
            let text_extractor = self.text_extractor.clone();

            self.processing_handles.push(tokio::spawn(async move {
                Indexer::process_queue(queue.as_ref(), url_processor, text_extractor).with_cancellation(&ct).await;
                info!("Indexing worker stopped");
            }.instrument(error_span!("indexing_worker", worker = i))));
        }
    }

    async fn process_queue(queue: &IndexingQueue, url_processor: U, text_extractor: TextExtractor) {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(Policy::limited(20))
            .connection_verbose(true)
            .build()
            .unwrap();

        let mut searching_client = loop {
            info!("Connecting to searching service...");
            match SearchingApiClient::connect("http://localhost:8083").await {
                Ok(client) => break client,
                Err(err) => {
                    warn!("Couldn't connect to searching service {}", err);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                },
            };
        };
        searching_client = searching_client.max_decoding_message_size(usize::MAX);

        let link_selector = Selector::parse("a").unwrap();
        let base_selector = Selector::parse("base").unwrap();

        loop {
            let queue_item = queue.dequeue().await;
            info!("Processing {}", queue_item);

            let html_text = match execute_request(&http_client, &queue_item).await {
                Ok(text) => text,
                Err(err) => {
                    warn!("Request {} failed {}", queue_item, err);
                    continue;
                },
            };

            let (links, text) = {
                let html = scraper::Html::parse_document(&html_text);
                if !html.errors.is_empty() {
                    warn!("Html has {} errors", html.errors.len());
                }

                let base_url = if let Some(base) = html.select(&base_selector).next() {
                    queue_item.join(base.value().attr("href").unwrap_or("")).unwrap_or(queue_item.clone())
                }
                else {
                    queue_item.clone()
                };

                let links = html.select(&link_selector)
                    .filter_map(|a| a.value().attr("href").and_then(|h| url_processor.parse_url(&base_url, h)))
                    .collect::<Vec<_>>();

                (links, text_extractor.extract_text(&html))
            };

            info!("Html has {} links", links.len());

            for link in links {
                queue.enqueue(link).await;
            }

            if let Some(text) = text {
                while let Err(err) = searching_client.add_page(AddPageRequest { url: queue_item.to_string(), text: text.clone() }).await {
                    warn!("Failed to send page to searching service {}", err);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}

async fn execute_request(client: &reqwest::Client, uri: &Url) -> reqwest::Result<String> {
    client.get(uri.clone()).send().await?.error_for_status()?.text().await
}

impl<U> Drop for Indexer<U> {
    fn drop(&mut self) {
        self.cancellation_token.cancel();

        for handle in self.processing_handles.iter() {
            while !handle.is_finished() {}
        }
    }
}