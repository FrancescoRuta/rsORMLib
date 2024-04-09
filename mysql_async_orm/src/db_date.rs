use mysql_async::prelude::FromValue;

#[derive(Debug, Clone)]
pub struct DBTime(i16, u8, u8);
#[derive(Debug, Clone)]
pub struct DBDate(i32, u8, u8);
#[derive(Debug, Clone)]
pub struct DBDateTime(i32, u8, u8, u8, u8, u8);

impl FromValue for DBTime {
	type Intermediate = DBTime;
}

impl TryFrom<mysql_async::Value> for DBTime {
	type Error = mysql_async::FromValueError;
	
	fn try_from(value: mysql_async::Value) -> Result<Self, Self::Error> {
		if let mysql_async::Value::Time(is_negative, d, h, m, s, _) = value {
			let h = if is_negative { -(h as i16) } else { h as i16 };
			Ok(DBTime(d as i16 * 24 + h, m, s))
		} else {
			Err(mysql_async::FromValueError(value))
		}
	}
}

impl From<DBTime> for mysql_async::Value {
	fn from(v: DBTime) -> Self {
		let DBTime(h, m, s) = v;
		let is_negative = h.is_negative();
		let h = h.abs() as u32;
		mysql_async::Value::Time(is_negative, h, m, s, 0, 0)
	}
}

impl TryFrom<mysql_async::Value> for DBDate {
	type Error = mysql_async::FromValueError;
	
	fn try_from(value: mysql_async::Value) -> Result<Self, Self::Error> {
		if let mysql_async::Value::Date(y, m, d, _, _, _, _) = value {
			Ok(DBDate(y as i32, m, d))
		} else {
			Err(mysql_async::FromValueError(value))
		}
	}
}

impl From<DBDate> for mysql_async::Value {
	fn from(v: DBDate) -> Self {
		let DBDate(y, m, d) = v;
		let y = if y.is_negative() { 0 } else { y };
		mysql_async::Value::Date(y as u16, m , d, 0, 0, 0, 0)
	}
}


impl FromValue for DBDateTime {
	type Intermediate = DBDateTime;
}

impl TryFrom<mysql_async::Value> for DBDateTime {
	type Error = mysql_async::FromValueError;
	
	fn try_from(value: mysql_async::Value) -> Result<Self, Self::Error> {
		if let mysql_async::Value::Date(y, m, d, h, min, s, _) = value {
			Ok(DBDateTime(y as i32, m, d, h, min, s))
		} else {
			Err(mysql_async::FromValueError(value))
		}
	}
}

impl From<DBDateTime> for mysql_async::Value {
	fn from(v: DBDateTime) -> Self {
		let DBDateTime(y, m, d, h, min, s) = v;
		let y = if y.is_negative() { 0 } else { y };
		mysql_async::Value::Date(y as u16, m, d, h, min, s, 0)
	}
}
