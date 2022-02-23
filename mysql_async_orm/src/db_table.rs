use crate::db_connection::{DbConnection, DbError};

pub trait DbTable<K> where Self: Sized {
	type DataCollector: DbTableDataCollector;
	fn vec_from_rows(rows: Vec<mysql_async::Row>) -> Vec<Self>;
	fn get_by_pk(pk: K, connection: &mut DbConnection) -> Option<Self>;
	fn save(&self, connection: &mut DbConnection) -> Result<(), DbError>;
}

pub trait DbTableDataCollector where Self: Sized {
	type Item;
	const SIZE: usize;
	fn sql() -> (&'static str, String, Vec<usize>, Vec<(String, String)>);
	fn new(offset: usize) -> Self;
	fn push_next(&mut self, next_row: &mut mysql_async::Row) -> Option<()>;
	fn build(self) -> Vec<Self::Item>;
}