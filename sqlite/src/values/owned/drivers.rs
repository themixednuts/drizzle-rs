//! Database driver implementations for `OwnedSQLiteValue`

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
            Self::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            Self::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*i),
            )),
            Self::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*f),
            )),
            Self::Text(s) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(s.as_bytes()),
            )),
            Self::Blob(b) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(b.as_ref()),
            )),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl rusqlite::types::FromSql for OwnedSQLiteValue {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let result = match value {
            rusqlite::types::ValueRef::Null => Self::Null,
            rusqlite::types::ValueRef::Integer(i) => Self::Integer(i),
            rusqlite::types::ValueRef::Real(r) => Self::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                Self::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => Self::Blob(items.to_vec().into_boxed_slice()),
        };
        Ok(result)
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::Value> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::Value) -> Self {
        match value {
            rusqlite::types::Value::Null => Self::Null,
            rusqlite::types::Value::Integer(i) => Self::Integer(i),
            rusqlite::types::Value::Real(r) => Self::Real(r),
            rusqlite::types::Value::Text(s) => Self::Text(s),
            rusqlite::types::Value::Blob(b) => Self::Blob(b.into_boxed_slice()),
        }
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::types::ValueRef<'_>> for OwnedSQLiteValue {
    fn from(value: rusqlite::types::ValueRef<'_>) -> Self {
        match value {
            rusqlite::types::ValueRef::Null => Self::Null,
            rusqlite::types::ValueRef::Integer(i) => Self::Integer(i),
            rusqlite::types::ValueRef::Real(r) => Self::Real(r),
            rusqlite::types::ValueRef::Text(items) => {
                Self::Text(String::from_utf8_lossy(items).into_owned())
            }
            rusqlite::types::ValueRef::Blob(items) => Self::Blob(items.to_vec().into_boxed_slice()),
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
        Self::from(SQLiteValue::from(value))
    }
}

#[cfg(feature = "turso")]
impl From<&OwnedSQLiteValue> for turso::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        Self::from(SQLiteValue::from(value))
    }
}

//------------------------------------------------------------------------------
// libsql implementations
//------------------------------------------------------------------------------

#[cfg(feature = "libsql")]
impl From<OwnedSQLiteValue> for libsql::Value {
    fn from(value: OwnedSQLiteValue) -> Self {
        Self::from(SQLiteValue::from(value))
    }
}

#[cfg(feature = "libsql")]
impl From<&OwnedSQLiteValue> for libsql::Value {
    fn from(value: &OwnedSQLiteValue) -> Self {
        Self::from(SQLiteValue::from(value))
    }
}
