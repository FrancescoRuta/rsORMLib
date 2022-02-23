use mysql_async_orm::DbTable;


#[derive(DbTable)]
#[from("clienti")]
struct Cliente {
	#[pk]
	id: Option<u32>,
}

fn main() {
	
}
