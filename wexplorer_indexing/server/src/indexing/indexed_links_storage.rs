use std::sync::{Arc, Mutex};

use chrono::{Utc, DateTime};
use rusqlite::{Connection, params, OptionalExtension, ToSql};
use sea_query::{Table, Iden, ColumnDef, Query, Value, SimpleExpr, Expr};
use url::Url;

use super::{SqliteSchemaStatementBuilder, SqliteQueryStatementWriter};

#[derive(Iden)]
enum IndexedLinks {
    Table,
    Url,
    LastIndexedTimestamp,
}

pub struct Storage {
    connection: Arc<Mutex<Connection>>,
    add_sql: String,
    get_last_indexed_timestamp_sql: String,
}

impl Storage {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Result<Self, rusqlite::Error> {
        let create_table_sql = Table::create()
            .table(IndexedLinks::Table)
            .if_not_exists()
            .col(ColumnDef::new(IndexedLinks::Url).text().not_null().primary_key())
            .col(ColumnDef::new(IndexedLinks::LastIndexedTimestamp).integer().not_null())
            .to_sqlite_string();
        connection.lock().unwrap().execute(&create_table_sql, ())?;

        let add_sql = Query::insert()
            .into_table(IndexedLinks::Table)
            .columns([IndexedLinks::Url, IndexedLinks::LastIndexedTimestamp])
            .values_panic([SimpleExpr::Custom("?1".to_string()), SimpleExpr::Custom("?2".to_string())])
            .to_sqlite_string()
            .replace("INSERT", "REPLACE");

        let get_last_indexed_timestamp_sql = Query::select()
            .column(IndexedLinks::LastIndexedTimestamp)
            .from(IndexedLinks::Table)
            .and_where(Expr::col(IndexedLinks::Url).eq(SimpleExpr::Custom("?1".to_string())))
            .to_sqlite_string();

        Ok(Self {
            connection,
            add_sql,
            get_last_indexed_timestamp_sql,
        })
    }

    pub fn get_last_indexed_time(&self, url: &Url) -> Result<Option<DateTime<Utc>>, rusqlite::Error> {
        self.connection.lock().unwrap()
            .query_row(&self.get_last_indexed_timestamp_sql, [url], |row| row.get(0))
            .optional()
    }

    pub fn add(&self, url: &Url, indexed_time: DateTime<Utc>) -> Result<(), rusqlite::Error> {
        self.connection.lock().unwrap().execute(&self.add_sql, params![url, indexed_time])?;
        Ok(())
    }
}