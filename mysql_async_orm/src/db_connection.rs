use std::fmt::Display;

use crate::db_table::DbTable;

pub struct DbConnection;
#[derive(Debug)]
pub struct DbError {
	message: String,
}

impl Display for DbError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}
impl std::error::Error for DbError { }

impl DbConnection {
	
	pub fn get_obj<K, T: DbTable<K>>(&mut self, pk: K) -> Option<T> {
		T::get_by_pk(pk, self)
	}
	
	pub fn save<K, T: DbTable<K>>(&mut self, obj: &T) -> Result<(), DbError> {
		obj.save(self)
	}
	
}