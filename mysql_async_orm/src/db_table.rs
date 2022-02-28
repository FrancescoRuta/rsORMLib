pub trait DbTable where Self: Sized {
	type DataCollector: DbTableDataCollector;
	fn prepare_insert(fk: Option<&str>, data: &Self, query: &mut String, this_id: usize);
	fn prepare_update(fk: Option<&str>, data: &Self, query: &mut String, this_id: usize);
	fn prepare_delete(fk: Option<&str>, data: &Self, query: &mut String, this_id: usize);
}

pub trait DbTableDataCollector where Self: Sized {
	type Item;
	const SIZE: usize;
	fn sql() -> (&'static str, String, Vec<usize>, Vec<(String, String)>);
	fn new(offset: usize) -> Self;
	fn push_next(&mut self, next_row: &mut mysql_async::Row) -> Option<()>;
	fn build(self) -> Vec<Self::Item>;
}