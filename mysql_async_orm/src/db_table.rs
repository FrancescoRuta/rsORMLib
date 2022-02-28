pub trait DbTable where Self: Sized {
	type DataCollector: DbTableDataCollector;
}

pub trait DbTableDataCollector where Self: Sized {
	type Item;
	const SIZE: usize;
	fn sql() -> (&'static str, String, Vec<usize>, Vec<(String, String)>);
	fn new(offset: usize) -> Self;
	fn push_next(&mut self, next_row: &mut mysql_async::Row) -> Option<()>;
	fn build(self) -> Vec<Self::Item>;
	fn get_insert_instr_as_sub<K: Into<mysql_async::Value> + Copy>(fk: &str) -> (String, fn(&Vec<Self::Item>, K) -> Vec<Vec<mysql_async::Value>>);
}