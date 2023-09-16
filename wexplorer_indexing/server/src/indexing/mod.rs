mod indexer;
mod url_processing;
mod text_extracting;
mod indexed_links_storage;

pub use indexer::*;
pub use url_processing::*;
pub use text_extracting::*;
pub use indexed_links_storage::*;

use sea_query::{SchemaStatementBuilder, SqliteQueryBuilder, QueryStatementWriter};

pub trait SqliteSchemaStatementBuilder: SchemaStatementBuilder {
    fn to_sqlite_string(&self) -> String {
        self.to_string(SqliteQueryBuilder {})
    }
}

pub trait SqliteQueryStatementWriter: QueryStatementWriter {
    fn to_sqlite_string(&self) -> String {
        self.to_string(SqliteQueryBuilder {})
    }
}

impl<T: SchemaStatementBuilder> SqliteSchemaStatementBuilder for T {}

impl<T: QueryStatementWriter> SqliteQueryStatementWriter for T {}