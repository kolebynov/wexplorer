use std::{future::Future, collections::{VecDeque, HashSet}, sync::Mutex};
use const_format::formatcp;

use itertools::Itertools;
use rusqlite::{Connection, OptionalExtension, Statement};
use tokio::sync::Notify;
use url::Url;
use crate::indexing::Indexer;

#[derive(PartialEq)]
pub struct QueueItem {
    pub id: i64,
    pub uri: Url
}

struct QueueItemStatus {}

impl QueueItemStatus {
    const READY: i32 = 0;

    const IN_PROGRESS: i32 = 1;
}

pub struct IndexingQueue {
    connection: Mutex<Connection>,
    new_item_notify: Notify,
}

impl IndexingQueue {
    const ID_COLUMN: &'static str = "Id";

    const URL_COLUMN: &'static str = "Url";

    const STATUS_COLUMN: &'static str = "Status";

    const TABLE_NAME: &'static str = "IndexingQueue";

    const CREATE_TABLE_SQL: &'static str = formatcp!(
        r#"CREATE TABLE IF NOT EXISTS "{}" (
            "{}" INTEGER NOT NULL,
            "{}" TEXT NOT NULL,
            "{}" INTEGER NOT NULL,
            PRIMARY KEY("{}" AUTOINCREMENT)
        );"#,
        IndexingQueue::TABLE_NAME, IndexingQueue::ID_COLUMN, IndexingQueue::URL_COLUMN,
        IndexingQueue::STATUS_COLUMN, IndexingQueue::ID_COLUMN
    );

    const RESET_IN_PROGRESS_ITEMS_SQL: &'static str = formatcp!(
        r#"UPDATE {} SET Status={} WHERE Status={}"#,
        IndexingQueue::TABLE_NAME, QueueItemStatus::READY, QueueItemStatus::IN_PROGRESS
    );

    const ENQUEUE_ITEM_SQL: &'static str = formatcp!(
        r#"INSERT INTO {} ({}, {}) VALUES (?1, {})"#,
        IndexingQueue::TABLE_NAME, IndexingQueue::URL_COLUMN, IndexingQueue::STATUS_COLUMN,
        QueueItemStatus::READY
    );

    const PEEK_ITEM_SQL: &'static str = formatcp!(
        r#"SELECT {}, {} FROM {} WHERE {}={} ORDER BY {} LIMIT 1"#,
        IndexingQueue::ID_COLUMN, IndexingQueue::URL_COLUMN, IndexingQueue::TABLE_NAME,
        IndexingQueue::STATUS_COLUMN, QueueItemStatus::READY, IndexingQueue::ID_COLUMN
    );

    const SET_IN_PROGRESS_SQL: &'static str = formatcp!(
        r#"UPDATE {} SET {}={} WHERE {}=?1"#,
        IndexingQueue::TABLE_NAME, IndexingQueue::STATUS_COLUMN, QueueItemStatus::IN_PROGRESS,
        IndexingQueue::ID_COLUMN
    );

    const REMOVE_ITEM_SQL: &'static str = formatcp!(
        r#"DELETE FROM {} WHERE {}=?1"#,
        IndexingQueue::TABLE_NAME, IndexingQueue::ID_COLUMN
    );

    pub fn new(connection: Connection) -> Result<Self, rusqlite::Error> {
        connection.execute(IndexingQueue::CREATE_TABLE_SQL, ())?;
        connection.execute(IndexingQueue::RESET_IN_PROGRESS_ITEMS_SQL, ())?;

        Ok(Self {
            connection: Mutex::new(connection),
            new_item_notify: Notify::new()
        })
    }

    pub fn enqueue(&self, uri: Url) -> Result<(), rusqlite::Error> {
        self.connection.lock().unwrap().execute(IndexingQueue::ENQUEUE_ITEM_SQL, [uri.as_str()])?;
        self.new_item_notify.notify_one();
        Ok(())
    }

    pub async fn peek(&self) -> Result<QueueItem, rusqlite::Error> {
        loop {
            {
                let connection_guard = self.connection.lock().unwrap();
                let peek_result = connection_guard.query_row(
                    IndexingQueue::PEEK_ITEM_SQL, (),
                    |row| Ok(QueueItem {
                        id: row.get(0)?,
                        uri: Url::parse(row.get::<_, String>(1)?.as_str()).unwrap(),
                    })).optional()?;

                if let Some(item) = peek_result {
                    connection_guard.execute(IndexingQueue::SET_IN_PROGRESS_SQL, [item.id])?;
                    return Ok(item);
                }
            }

            self.new_item_notify.notified().await;
        }
    }

    pub fn mark_processed(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.connection.lock().unwrap().execute(IndexingQueue::REMOVE_ITEM_SQL, [id])?;
        Ok(())
    }

    pub async fn contains_authority(&self, authority: &str) -> bool {
        false
    }

    pub async fn get_indexing_origins(&self) -> Vec<String> {
        vec![]
    }

    pub async fn get_indexing_pages(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tokio::runtime::Handle;
    use tokio::task::block_in_place;
    use url::Url;
    use crate::indexing::Indexer;
    use crate::queue::IndexingQueue;

    #[test]
    fn test() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let connection = Connection::open("test.db").unwrap();
                let queue = IndexingQueue::new(connection).unwrap();
                queue.enqueue(Url::parse("http://localhost").unwrap()).unwrap();
                let item = queue.peek().await.unwrap();
                queue.mark_processed(item.id).unwrap();
                println!("123");
            });
    }
}