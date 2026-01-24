//! Database driver implementations for OwnedSQLiteValue

#[cfg(any(feature = "turso", feature = "libsql"))]
use super::super::SQLiteValue;
#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use super::OwnedSQLiteValue;

//------------------------------------------------------------------------------
// rusqlite implementations
//------------------------------------------------------------------------------

#[cfg(feature = "rusqlite")]
impl rusqlite::ToSql for OwnedSQLiteValue {
    fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
        match self {
            OwnedSQLiteValue::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            OwnedSQLiteValue::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*i),
            )),
            OwnedSQLiteValue::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*f),
            )),
            OwnedSQLiteValue::Text(s) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(s.as_bytes()),
            )),
            OwnedSQLiteValue::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(b.as_ref()),
            )),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl rusqlite::types::FromSql for OwnedSQLiteValue {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let result = match value {
            rusqlite::types::ValueRef::Null => OwnedSQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                OwnedSQLiteValue::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => {
                OwnedSQLiteValue::Blob(items.to_vec().into_boxed_slice())
            }
        };
        Ok(result)
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::Value> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::Value) -> Self {
        match value {
            rusqlite::types::Value::Null => OwnedSQLiteValue::Null,
            rusqlite::types::Value::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::Value::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::Value::Text(s) => OwnedSQLiteValue::Text(s),
            rusqlite::types::Value::Blob(b) => OwnedSQLiteValue::Blob(b.into_boxed_slice()),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::ValueRef<'_>> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::ValueRef<'_>) -> Self {
        match value {
            rusqlite::types::ValueRef::Null => OwnedSQLiteValue::Null,
            rusqlite::types::ValueRef::Integer(i) => OwnedSQLiteValue::Integer(i),
            rusqlite::types::ValueRef::Real(r) => OwnedSQLiteValue::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                OwnedSQLiteValue::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => {
                OwnedSQLiteValue::Blob(items.to_vec().into_boxed_slice())
            }
        }
    }
}

//------------------------------------------------------------------------------
// turso implementations
//------------------------------------------------------------------------------

#[cfg(feature = "turso")]
impl turso::IntoValue for OwnedSQLiteValue {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::from(self))
    }
}

#[cfg(feature = "turso")]
impl turso::IntoValue for &OwnedSQLiteValue {
    fn into_value(self) -> turso::Result<turso::Value> {
        Ok(turso::Value::from(self))
    }
}

#[cfg(feature = "turso")]
impl From<OwnedSQLiteValue> for turso::Value {
    fn from(value: OwnedSQLiteValue) -> Self {
        turso::Value::from(SQLiteValue::from(value))
    }
}

#[cfg(feature = "turso")]
impl From<&OwnedSQLiteValue> for turso::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        turso::Value::from(SQLiteValue::from(value))
    }
}

//------------------------------------------------------------------------------
// libsql implementations
//------------------------------------------------------------------------------

#[cfg(feature = "libsql")]
impl From<OwnedSQLiteValue> for libsql::Value {
    fn from(value: OwnedSQLiteValue) -> Self {
        libsql::Value::from(SQLiteValue::from(value))
    }
}

#[cfg(feature = "libsql")]
impl From<&OwnedSQLiteValue> for libsql::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        libsql::Value::from(SQLiteValue::from(value))
    }
}
