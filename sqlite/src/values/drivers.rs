//! Database driver implementations for SQLiteValue
//!
//! Contains implementations for rusqlite, turso, and libsql drivers.

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use super::SQLiteValue;
#[cfg(feature = "rusqlite")]
use std::borrow::Cow;

#[cfg(feature = "rusqlite")]
use rusqlite::types::FromSql;
#[cfg(feature = "turso")]
use turso::IntoValue;

//------------------------------------------------------------------------------
// rusqlite implementations
//------------------------------------------------------------------------------

#[cfg(feature = "rusqlite")]
impl<'a> rusqlite::ToSql for SQLiteValue<'a> {
    fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SQLiteValue::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            SQLiteValue::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*i),
            )),
            SQLiteValue::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*f),
            )),
            SQLiteValue::Text(s) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(s.as_bytes()),
            )),
            SQLiteValue::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(b.as_ref()),
            )),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> FromSql for SQLiteValue<'a> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let result = match value {
            rusqlite::types::ValueRef::Null => SQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => SQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                SQLiteValue::Text(String::from_utf8_lossy(items).into_owned().into())
            }
            rusqlite::types::ValueRef::Blob(items) => SQLiteValue::Blob(items.to_vec().into()),
        };
        Ok(result)
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> From<rusqlite::types::Value> for SQLiteValue<'a> {
    fn from(value: rusqlite::types::Value) -> Self {
        match value {
            rusqlite::types::Value::Null => SQLiteValue::Null,
            rusqlite::types::Value::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::Value::Real(r) => SQLiteValue::Real(r),
            rusqlite::types::Value::Text(s) => SQLiteValue::Text(s.into()),
            rusqlite::types::Value::Blob(b) => SQLiteValue::Blob(b.into()),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> From<rusqlite::types::ValueRef<'a>> for SQLiteValue<'a> {
    fn from(value: rusqlite::types::ValueRef<'a>) -> Self {
        match value {
            rusqlite::types::ValueRef::Null => SQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => SQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => SQLiteValue::Real(r),
            // Zero-copy: borrow if valid UTF-8, otherwise allocate for lossy conversion
            rusqlite::types::ValueRef::Text(items) => match std::str::from_utf8(items) {
                Ok(s) => SQLiteValue::Text(Cow::Borrowed(s)),
                Err(_) => SQLiteValue::Text(String::from_utf8_lossy(items).into_owned().into()),
            },
            // Zero-copy: borrow the blob slice directly
            rusqlite::types::ValueRef::Blob(items) => SQLiteValue::Blob(Cow::Borrowed(items)),
        }
    }
}

//------------------------------------------------------------------------------
// turso implementations
//------------------------------------------------------------------------------

#[cfg(feature = "turso")]
impl<'a> IntoValue for SQLiteValue<'a> {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::from(self))
    }
}

#[cfg(feature = "turso")]
impl<'a> IntoValue for &SQLiteValue<'a> {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::from(self))
    }
}

#[cfg(feature = "turso")]
impl<'a> From<SQLiteValue<'a>> for turso::Value {
    fn from(value: SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(i),
            SQLiteValue::Real(r) => turso::Value::Real(r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.into_owned()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.into_owned()),
            SQLiteValue::Null => turso::Value::Null,
        }
    }
}

#[cfg(feature = "turso")]
impl<'a> From<&SQLiteValue<'a>> for turso::Value {
    fn from(value: &SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => turso::Value::Integer(*i),
            SQLiteValue::Real(r) => turso::Value::Real(*r),
            SQLiteValue::Text(cow) => turso::Value::Text(cow.clone().into_owned()),
            SQLiteValue::Blob(cow) => turso::Value::Blob(cow.clone().into_owned()),
            SQLiteValue::Null => turso::Value::Null,
        }
    }
}

//------------------------------------------------------------------------------
// libsql implementations
//------------------------------------------------------------------------------

#[cfg(feature = "libsql")]
impl<'a> From<SQLiteValue<'a>> for libsql::Value {
    fn from(value: SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => libsql::Value::Integer(i),
            SQLiteValue::Real(r) => libsql::Value::Real(r),
            SQLiteValue::Text(cow) => libsql::Value::Text(cow.into_owned()),
            SQLiteValue::Blob(cow) => libsql::Value::Blob(cow.into_owned()),
            SQLiteValue::Null => libsql::Value::Null,
        }
    }
}

#[cfg(feature = "libsql")]
impl<'a> From<&SQLiteValue<'a>> for libsql::Value {
    fn from(value: &SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(i) => libsql::Value::Integer(*i),
            SQLiteValue::Real(r) => libsql::Value::Real(*r),
            SQLiteValue::Text(cow) => libsql::Value::Text(cow.clone().into_owned()),
            SQLiteValue::Blob(cow) => libsql::Value::Blob(cow.clone().into_owned()),
            SQLiteValue::Null => libsql::Value::Null,
        }
    }
}
