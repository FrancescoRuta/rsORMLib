use std::{future::Future, pin::Pin};

use mysql_async::{
	prelude::{FromRow, Queryable, StatementLike},
	TxOpts,
};

pub struct DbConnectionPool {
	pool: mysql_async::Pool,
}

pub struct DbConnection {
	conn: mysql_async::Conn,
}
pub struct DbTransaction<'a> {
	conn: mysql_async::Transaction<'a>,
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

impl<'a> DbTransaction<'a> {
	pub async fn commit(self) -> Result<(), DbError> {
		self.conn.commit().await
	}

	pub async fn rollback(self) -> Result<(), DbError> {
		self.conn.rollback().await
	}
}

impl DbConnection {
	pub fn new(conn: mysql_async::Conn) -> Self {
		DbConnection { conn }
	}

	pub async fn start_transaction(&mut self) -> Result<DbTransaction<'_>, DbError> {
		Ok(DbTransaction {
			conn: self.conn.start_transaction(TxOpts::default()).await?,
		})
	}

	pub async fn query<Q: AsRef<str> + Send + Sync, R: FromRow + Send + 'static>(
		&mut self,
		query: Q,
	) -> Result<Vec<R>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query(query).await
	}

	pub async fn query_first<Q: AsRef<str> + Send + Sync, R: FromRow + Send + 'static>(
		&mut self,
		query: Q,
	) -> Result<Option<R>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query_first(query).await
	}

	pub async fn query_iter<'a, Q: AsRef<str> + Send + Sync + 'a>(
		&'a mut self,
		query: Q,
	) -> Result<mysql_async::QueryResult<'a, 'static, mysql_async::TextProtocol>, DbError> {
		println!("{}", query.as_ref());
		self.conn.query_iter(query).await
	}

	pub async fn query_drop<Q: AsRef<str> + Send + Sync>(
		&mut self,
		query: Q,
	) -> Result<(), DbError> {
		println!("{}", query.as_ref());
		self.conn.query_drop(query).await
	}

	pub async fn exec<
		S: StatementLike,
		P: Into<mysql_async::Params> + Send,
		R: FromRow + Send + 'static,
	>(
		&mut self,
		stmt: S,
		params: P,
	) -> Result<Vec<R>, DbError> {
		self.conn.exec(stmt, params).await
	}

	pub async fn exec_first<
		S: StatementLike,
		P: Into<mysql_async::Params> + Send,
		R: FromRow + Send + 'static,
	>(
		&mut self,
		stmt: S,
		params: P,
	) -> Result<Option<R>, DbError> {
		self.conn.exec_first(stmt, params).await
	}

	pub async fn exec_drop<
		S: StatementLike,
		P: Into<mysql_async::Params> + Send,
		R: FromRow + Send + 'static,
	>(
		&mut self,
		stmt: S,
		params: P,
	) -> Result<(), DbError> {
		self.conn.exec_drop(stmt, params).await
	}

	pub async fn exec_batch<S, P, R, I>(&mut self, stmt: S, params: I) -> Result<(), DbError>
	where
		S: StatementLike,
		I: IntoIterator<Item = P> + Send,
		I::IntoIter: Send,
		P: Into<mysql_async::Params> + Send,
	{
		self.conn.exec_batch(stmt, params).await
	}
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, DbError>> + Send + 'a>>;

pub trait QueryableConn {
	fn query<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<'a, Vec<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static;
	fn query_first<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<Option<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static;
	fn query_iter<'a, Q: AsRef<str> + Send + Sync + 'a>(
		&'a mut self,
		query: Q,
	) -> BoxFuture<mysql_async::QueryResult<'a, 'static, mysql_async::TextProtocol>>;
	fn query_drop<'a, Q>(&'a mut self, query: Q) -> BoxFuture<'a, ()>
	where
		Q: AsRef<str> + Send + Sync + 'a;
	fn exec<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Vec<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static;
	fn exec_first<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Option<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static;
	fn exec_drop<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static;
	fn exec_batch<'a, S, P, R, I>(&'a mut self, stmt: S, params: I) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		I: IntoIterator<Item = P> + Send + 'a,
		I::IntoIter: Send,
		P: Into<mysql_async::Params> + Send;
}

impl QueryableConn for DbConnection {
	fn query<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<'a, Vec<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.query(query)
	}

	fn query_first<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<Option<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.query_first(query)
	}

	fn query_iter<'a, Q: AsRef<str> + Send + Sync + 'a>(
		&'a mut self,
		query: Q,
	) -> BoxFuture<mysql_async::QueryResult<'a, 'static, mysql_async::TextProtocol>> {
		self.conn.query_iter(query)
	}

	fn query_drop<'a, Q>(&'a mut self, query: Q) -> BoxFuture<'a, ()>
	where
		Q: AsRef<str> + Send + Sync + 'a,
	{
		self.conn.query_drop(query)
	}

	fn exec<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Vec<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec(stmt, params)
	}

	fn exec_first<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Option<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec_first(stmt, params)
	}

	fn exec_drop<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec_drop(stmt, params)
	}

	fn exec_batch<'a, S, P, R, I>(&'a mut self, stmt: S, params: I) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		I: IntoIterator<Item = P> + Send + 'a,
		I::IntoIter: Send,
		P: Into<mysql_async::Params> + Send,
	{
		self.conn.exec_batch(stmt, params)
	}
}

impl QueryableConn for DbTransaction<'_> {
	fn query<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<'a, Vec<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.query(query)
	}

	fn query_first<'a, Q, R>(&'a mut self, query: Q) -> BoxFuture<Option<R>>
	where
		Q: AsRef<str> + Send + Sync + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.query_first(query)
	}

	fn query_iter<'a, Q: AsRef<str> + Send + Sync + 'a>(
		&'a mut self,
		query: Q,
	) -> BoxFuture<mysql_async::QueryResult<'a, 'static, mysql_async::TextProtocol>> {
		self.conn.query_iter(query)
	}

	fn query_drop<'a, Q>(&'a mut self, query: Q) -> BoxFuture<'a, ()>
	where
		Q: AsRef<str> + Send + Sync + 'a,
	{
		self.conn.query_drop(query)
	}

	fn exec<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Vec<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec(stmt, params)
	}

	fn exec_first<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, Option<R>>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec_first(stmt, params)
	}

	fn exec_drop<'a, S, P, R>(&'a mut self, stmt: S, params: P) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		P: Into<mysql_async::Params> + Send + 'a,
		R: FromRow + Send + 'static,
	{
		self.conn.exec_drop(stmt, params)
	}

	fn exec_batch<'a, S, P, R, I>(&'a mut self, stmt: S, params: I) -> BoxFuture<'a, ()>
	where
		S: StatementLike + 'a,
		I: IntoIterator<Item = P> + Send + 'a,
		I::IntoIter: Send,
		P: Into<mysql_async::Params> + Send,
	{
		self.conn.exec_batch(stmt, params)
	}
}
