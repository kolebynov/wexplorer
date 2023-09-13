use std::{future::Future, collections::{VecDeque, HashSet}, sync::Mutex};

use itertools::Itertools;
use tokio::sync::Notify;
use url::Url;

#[derive(PartialEq)]
struct QueueItem {
    uri: Url
}

pub struct IndexingQueue {
    queue: Mutex<VecDeque<QueueItem>>,
    new_item_notify: Notify,
}

impl IndexingQueue {
    pub fn new() -> Self {
        Self { queue: Mutex::new(VecDeque::new()), new_item_notify: Notify::new() }
    }

    pub async fn enqueue(&self, uri: Url) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let queue_item = QueueItem { uri };
        if queue.contains(&queue_item) {
            return false;
        }

        queue.push_back(queue_item);
        self.new_item_notify.notify_one();
        true
    }

    pub async fn dequeue(&self) -> Url {
        loop {
            if let Some(item) = self.queue.lock().unwrap().pop_front() {
                return item.uri;
            }
            self.new_item_notify.notified().await;
        }
    }

    pub async fn contains_authority(&self, authority: &str) -> bool {
        self.queue.lock().unwrap().iter().any(|item| item.uri.authority().eq(authority))
    }

    pub async fn get_indexing_origins(&self) -> Vec<String> {
        self.queue.lock().unwrap()
            .iter()
            .filter_map(|i| Some(format!("{}://{}", i.uri.scheme(), i.uri.authority())))
            .unique()
            .collect()
    }

    pub async fn get_indexing_pages(&self) -> Vec<String> {
        self.queue.lock().unwrap()
            .iter()
            .map(|i| i.uri.to_string())
            .collect()
    }
}