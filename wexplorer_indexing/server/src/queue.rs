use std::{future::Future, collections::{VecDeque, HashSet}, sync::{Mutex, Arc}};
use const_format::formatcp;

use itertools::Itertools;
use rusqlite::{Connection, OptionalExtension, Statement};
use sea_query::{Table, ColumnDef, SqliteQueryBuilder, Iden, Query, Expr, Value, QueryStatementWriter, SchemaStatementBuilder, QueryStatementBuilder, Order, SimpleExpr};
use tokio::sync::Notify;
use tracing::{info, debug};
use url::Url;
use crate::indexing::{Indexer, SqliteSchemaStatementBuilder, SqliteQueryStatementWriter};

#[derive(PartialEq)]
pub struct QueueItem {
    pub id: i64,
    pub url: Url
}

struct QueueItemStatus {}

impl QueueItemStatus {
    const READY: i32 = 0;

    const IN_PROGRESS: i32 = 1;
}

#[derive(Iden)]
enum Queue {
    Table,
    Id,
    Url,
    Status,
}

pub struct IndexingQueue {
    connection: Arc<Mutex<Connection>>,
    enqueue_item_sql: String,
    peek_item_sql: String,
    set_in_progress_sql: String,
    remove_item_sql: String,
    new_item_notify: Notify,
}

impl IndexingQueue {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Result<Self, rusqlite::Error> {
        {
            let connection_guard = connection.lock().unwrap();
            let create_table_sql = Table::create()
                .table(Queue::Table)
                .if_not_exists()
                .col(ColumnDef::new(Queue::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Queue::Url).text().not_null().unique_key())
                .col(ColumnDef::new(Queue::Status).integer().not_null())
                .to_sqlite_string();
            let reset_in_progress_items_sql = Query::update()
                .table(Queue::Table)
                .value(Queue::Status, QueueItemStatus::READY)
                .and_where(Expr::col(Queue::Status).eq(QueueItemStatus::IN_PROGRESS))
                .to_sqlite_string();
            connection_guard.execute(&create_table_sql, ())?;
            connection_guard.execute(&reset_in_progress_items_sql, ())?;
        }

        let enqueue_item_sql = Query::insert()
            .into_table(Queue::Table)
            .columns([Queue::Url, Queue::Status])
            .values_panic([SimpleExpr::Custom("?1".to_string()), QueueItemStatus::READY.into()])
            .to_sqlite_string()
            .replace("INSERT", "INSERT OR IGNORE");

        let peek_item_sql = Query::select()
            .columns([Queue::Id, Queue::Url])
            .from(Queue::Table)
            .and_where(Expr::col(Queue::Status).eq(QueueItemStatus::READY))
            .order_by(Queue::Id, Order::Asc)
            .limit(1)
            .to_sqlite_string();

        let set_in_progress_sql = Query::update()
            .table(Queue::Table)
            .value(Queue::Status, QueueItemStatus::IN_PROGRESS)
            .and_where(Expr::col(Queue::Id).eq(SimpleExpr::Custom("?1".to_string())))
            .to_sqlite_string();

        let remove_item_sql = Query::delete()
            .from_table(Queue::Table)
            .and_where(Expr::col(Queue::Id).eq(SimpleExpr::Custom("?1".to_string())))
            .to_sqlite_string();

        Ok(Self {
            connection,
            enqueue_item_sql,
            peek_item_sql,
            set_in_progress_sql,
            remove_item_sql,
            new_item_notify: Notify::new(),
        })
    }

    pub fn enqueue(&self, url: Url) -> Result<bool, rusqlite::Error> {
        let inserted = self.connection.lock().unwrap().execute(&self.enqueue_item_sql, [&url])? > 0;

        if inserted {
            self.new_item_notify.notify_one();
        }
        else {
            debug!("Duplicated URL {} was not added to the indexing queue", url);
        }

        Ok(inserted)
    }

    pub async fn peek(&self) -> Result<QueueItem, rusqlite::Error> {
        loop {
            {
                let connection_guard = self.connection.lock().unwrap();
                let peek_result = connection_guard.query_row(
                    &self.peek_item_sql, (),
                    |row| Ok(QueueItem {
                        id: row.get(0)?,
                        url: row.get::<_, Url>(1)?,
                    })).optional()?;

                if let Some(item) = peek_result {
                    connection_guard.execute(&self.set_in_progress_sql, [item.id])?;
                    return Ok(item);
                }
            }

            self.new_item_notify.notified().await;
        }
    }

    pub fn mark_processed(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.connection.lock().unwrap().execute(&self.remove_item_sql, [id])?;
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
    use std::sync::{Arc, Mutex};

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
                let queue = IndexingQueue::new(Arc::new(Mutex::new(connection))).unwrap();
                queue.enqueue(Url::parse("http://localhost").unwrap()).unwrap();
                let item = queue.peek().await.unwrap();
                queue.mark_processed(item.id).unwrap();
                println!("123");
            });
    }
}