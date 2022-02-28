pub struct DbConnectionPool {
	pool: mysql_async::Pool,
}
pub struct DbConnection {
	conn: mysql_async::Conn,
}
pub type DbError = mysql_async::Error;

impl DbConnectionPool {
	pub fn new<O>(opts: O) -> Self
	where
		mysql_async::Opts: TryFrom<O>,
		<mysql_async::Opts as TryFrom<O>>::Error: std::error::Error,
	{
		DbConnectionPool {
			pool: mysql_async::Pool::new(opts),
		}
	}
	pub async fn get_conn(&self) -> Result<DbConnection, DbError> {
		Ok(DbConnection::new(self.pool.get_conn().await?))
	}
	pub async fn disconnect(self) -> Result<(), DbError> {
		self.pool.disconnect().await
	}
}
use mysql_async::prelude::{Queryable, StatementLike, FromRow};

impl DbConnection {
	
	fn new(conn: mysql_async::Conn) -> Self {
		DbConnection { conn }
	}
	
	pub async fn query<Q: AsRef<str> + Send + Sync, T: FromRow + Send + 'static>(&mut self, query: Q) -> Result<Vec<T>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query(query).await
	}
	
	pub async fn query_first<Q: AsRef<str> + Send + Sync, T: FromRow + Send + 'static>(&mut self, query: Q) -> Result<Option<T>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query_first(query).await
	}
	
	pub async fn query_iter<'a, Q: AsRef<str> + Send + Sync + 'a>(&'a mut self, query: Q) -> Result<mysql_async::QueryResult<'a, 'static, mysql_async::TextProtocol>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query_iter(query).await
	}
	
	pub async fn query_drop<Q: AsRef<str> + Send + Sync>(&mut self, query: Q) -> Result<(), DbError> {
		println!("{}", query.as_ref());
		self.conn.query_drop(query).await
	}
	
	pub async fn exec<S: StatementLike, P: Into<mysql_async::Params> + Send, T: FromRow + Send + 'static>(&mut self, stmt: S, params: P) -> Result<Vec<T>, DbError> {
		self.conn.exec(stmt, params).await
	}
	
	pub async fn exec_first<S: StatementLike, P: Into<mysql_async::Params> + Send, T: FromRow + Send + 'static>(&mut self, stmt: S, params: P) -> Result<Option<T>, DbError> {
		self.conn.exec_first(stmt, params).await
	}
	
	pub async fn exec_drop<S: StatementLike, P: Into<mysql_async::Params> + Send, T: FromRow + Send + 'static>(&mut self, stmt: S, params: P) -> Result<(), DbError> {
		self.conn.exec_drop(stmt, params).await
	}
	
	pub async fn exec_batch<S, P, T, I>(&mut self, stmt: S, params: I) -> Result<(), DbError>
	where
		S: StatementLike,
		I: IntoIterator<Item = P> + Send,
		I::IntoIter: Send,
		P: Into<mysql_async::Params> + Send
	{
		self.conn.exec_batch(stmt, params).await
	}
	
}