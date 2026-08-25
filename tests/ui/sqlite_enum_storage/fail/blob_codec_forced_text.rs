use drizzle::error::DrizzleError;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::traits::DrizzleSQLiteColumn;
use std::borrow::Cow;

#[derive(Clone, Debug, Default, PartialEq)]
struct BlobValue(Vec<u8>);

impl DrizzleSQLiteColumn for BlobValue {
    type SQLType = drizzle::sqlite::types::Blob;

    fn decode(value: SQLiteValueRef<'_>) -> Result<Self, DrizzleError> {
        match value {
            SQLiteValueRef::Blob(value) => Ok(Self(value.to_vec())),
            _ => Err(DrizzleError::ConversionError("expected BLOB".into())),
        }
    }

    fn encode(&self) -> SQLiteValue<'_> {
        SQLiteValue::Blob(Cow::Borrowed(&self.0))
    }
}

#[SQLiteTable]
struct Records {
    id: i64,
    #[column(text)]
    value: BlobValue,
}

fn main() {}
