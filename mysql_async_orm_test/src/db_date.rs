use mysql_async_orm::mysql_async::prelude::*;


#[derive(Debug, Clone)]
pub struct DBTime(i16, u8, u8);
#[derive(Debug, Clone)]
pub struct DBDate(i32, u8, u8);
#[derive(Debug, Clone)]
pub struct DBDateTime(i32, u8, u8, u8, u8, u8);

impl FromValue for DBTime {
	type Intermediate = DBTime;
	
	fn from_value_opt(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		Ok(
			if let mysql_async_orm::mysql_async::Value::Time(is_negative, d, h, m, s, _) = v {
				let h = if is_negative { -(h as i16) } else { h as i16 };
				DBTime(d as i16 * 24 + h, m, s)
			} else {
				let ir = <Self::Intermediate as ConvIr<Self>>::new(v)?;
				ir.commit()
			}
		)
	}
	
}

impl From<DBTime> for mysql_async_orm::mysql_async::Value {
	fn from(v: DBTime) -> Self {
		let DBTime(h, m, s) = v;
		let is_negative = h.is_negative();
		let h = h.abs() as u32;
		mysql_async_orm::mysql_async::Value::Time(is_negative, h, m, s, 0, 0)
	}
}

impl ConvIr<DBTime> for DBTime {
	fn new(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		if let mysql_async_orm::mysql_async::Value::Time(is_negative, _, hours, minutes, seconds, _) = v {
			let hours = if is_negative { -(hours as i16) } else { hours as i16 };
			Ok(DBTime(hours, minutes, seconds))
		} else {
			Err(mysql_async_orm::mysql_async::FromValueError(v))
		}
	}
	fn commit(self) -> DBTime {
		self.into()
	}
	fn rollback(self) -> mysql_async_orm::mysql_async::Value {
		self.to_value()
	}
}

impl FromValue for DBDate {
	type Intermediate = DBDate;
	
	fn from_value_opt(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		Ok(
			if let mysql_async_orm::mysql_async::Value::Date(y, m, d, _, _, _, _) = v {
				DBDate(y as i32, m, d)
			} else {
				let ir = Self::Intermediate::new(v)?;
				ir.commit()
			}
		)
	}
	
}

impl From<DBDate> for mysql_async_orm::mysql_async::Value {
	fn from(v: DBDate) -> Self {
		let DBDate(y, m, d) = v;
		let y = if y.is_negative() { 0 } else { y };
		mysql_async_orm::mysql_async::Value::Date(y as u16, m , d, 0, 0, 0, 0)
	}
}

impl ConvIr<DBDate> for DBDate {
	fn new(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		if let mysql_async_orm::mysql_async::Value::Date(year, month, day, _, _, _, _) = v {
			Ok(DBDate(year as i32, month, day))
		} else {
			Err(mysql_async_orm::mysql_async::FromValueError(v))
		}
	}
	fn commit(self) -> DBDate {
		self.into()
	}
	fn rollback(self) -> mysql_async_orm::mysql_async::Value {
		self.to_value()
	}
}

impl FromValue for DBDateTime {
	type Intermediate = DBDateTime;
	
	fn from_value_opt(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		Ok(
			if let mysql_async_orm::mysql_async::Value::Date(y, m, d, h, min, s, _) = v {
				DBDateTime(y as i32, m, d, h, min, s)
			} else {
				let ir = <Self::Intermediate as ConvIr<Self>>::new(v)?;
				ir.commit()
			}
		)
	}
	
}

impl From<DBDateTime> for mysql_async_orm::mysql_async::Value {
	fn from(v: DBDateTime) -> Self {
		let DBDateTime(y, m, d, h, min, s) = v;
		let y = if y.is_negative() { 0 } else { y };
		mysql_async_orm::mysql_async::Value::Date(y as u16, m, d, h, min, s, 0)
	}
}

impl ConvIr<DBDateTime> for DBDateTime {
	fn new(v: mysql_async_orm::mysql_async::Value) -> Result<Self, mysql_async_orm::mysql_async::FromValueError> {
		if let mysql_async_orm::mysql_async::Value::Date(year, month, day, hour, minutes, seconds, _) = v {
			Ok(DBDateTime(year as i32, month, day, hour, minutes, seconds))
		} else {
			Err(mysql_async_orm::mysql_async::FromValueError(v))
		}
	}
	fn commit(self) -> DBDateTime {
		self.into()
	}
	fn rollback(self) -> mysql_async_orm::mysql_async::Value {
		self.to_value()
	}
}
