pub trait DbTable where Self: Sized {
	type DataCollector: DbTableDataCollector;
	type PrimaryKey;
	fn prepare_insert(fk: Option<&str>, data: &Self, query: &mut String, this_id: usize);
	fn prepare_update(fk: Option<&str>, new_data: &Self, old_data: &Self, query: &mut String, this_id: usize);
	fn prepare_delete(fk: Option<&str>, data: &Self, query: &mut String, this_id: usize);
	fn get_pk(&self) -> Option<Self::PrimaryKey>;
}

pub trait DbTableDataCollector where Self: Sized {
	type Item;
	const SIZE: usize;
	fn sql() -> (&'static str, String, Vec<usize>, Vec<(&'static str, String, String)>);
	fn new(offset: usize) -> Self;
	fn push_next(&mut self, next_row: &mut mysql_async::Row) -> Option<()>;
	fn build(self) -> Vec<Self::Item>;
}