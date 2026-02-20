//! Database driver implementations for PostgresValue

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use super::PostgresValue;

//------------------------------------------------------------------------------
// postgres/tokio-postgres ToSql implementations
// Both crates use the same postgres-types underneath, so we only need one implementation
//------------------------------------------------------------------------------

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
mod postgres_tosql_impl {
    use super::PostgresValue;

    // Import from whichever crate is available
    #[cfg(feature = "postgres-sync")]
    use postgres::types::{IsNull, ToSql, Type};

    #[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
    use tokio_postgres::types::{IsNull, ToSql, Type};

    use bytes::BytesMut;

    impl<'a> ToSql for PostgresValue<'a> {
        fn to_sql(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
            match self {
                PostgresValue::Null => Ok(IsNull::Yes),
                PostgresValue::Smallint(i) => i.to_sql(ty, out),
                PostgresValue::Integer(i) => i.to_sql(ty, out),
                PostgresValue::Bigint(i) => i.to_sql(ty, out),
                PostgresValue::Real(f) => f.to_sql(ty, out),
                PostgresValue::DoublePrecision(f) => f.to_sql(ty, out),
                #[cfg(feature = "rust-decimal")]
                PostgresValue::Numeric(d) => d.to_sql(ty, out),
                PostgresValue::Text(cow) => cow.as_ref().to_sql(ty, out),
                PostgresValue::Bytea(cow) => cow.as_ref().to_sql(ty, out),
                PostgresValue::Boolean(b) => b.to_sql(ty, out),
                #[cfg(feature = "uuid")]
                PostgresValue::Uuid(uuid) => uuid.to_sql(ty, out),
                #[cfg(feature = "serde")]
                PostgresValue::Json(json) => json.to_sql(ty, out),
                #[cfg(feature = "serde")]
                PostgresValue::Jsonb(json) => json.to_sql(ty, out),
                #[cfg(feature = "chrono")]
                PostgresValue::Timestamp(ts) => ts.to_sql(ty, out),
                #[cfg(feature = "chrono")]
                PostgresValue::Date(date) => date.to_sql(ty, out),
                #[cfg(feature = "chrono")]
                PostgresValue::Time(time) => time.to_sql(ty, out),
                #[cfg(feature = "chrono")]
                PostgresValue::TimestampTz(ts) => ts.to_sql(ty, out),
                #[cfg(feature = "chrono")]
                PostgresValue::Interval(dur) => dur.to_string().to_sql(ty, out),
                #[cfg(feature = "cidr")]
                PostgresValue::Inet(ip) => ip.to_sql(ty, out),
                #[cfg(feature = "cidr")]
                PostgresValue::Cidr(ip) => ip.to_sql(ty, out),
                // MAC addresses don't have native ToSql in postgres-rs, use eui48 crate or string format
                #[cfg(feature = "cidr")]
                PostgresValue::MacAddr(mac) => format!(
                    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                )
                .to_sql(ty, out),
                #[cfg(feature = "cidr")]
                PostgresValue::MacAddr8(mac) => format!(
                    "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], mac[6], mac[7]
                )
                .to_sql(ty, out),
                // Point has native ToSql in postgres-rs with geo-types feature
                #[cfg(feature = "geo-types")]
                PostgresValue::Point(p) => p.to_sql(ty, out),
                // LineString maps to PATH in postgres, which has native support
                #[cfg(feature = "geo-types")]
                PostgresValue::LineString(ls) => ls.to_sql(ty, out),
                // Rect maps to BOX in postgres, which has native support
                #[cfg(feature = "geo-types")]
                PostgresValue::Rect(rect) => rect.to_sql(ty, out),
                #[cfg(feature = "bit-vec")]
                PostgresValue::BitVec(bits) => bits.to_sql(ty, out),
                PostgresValue::Enum(enum_val) => enum_val.variant_name().to_sql(ty, out),
                PostgresValue::Array(arr) => {
                    // For arrays, we need to serialize each element
                    // This is a simplified version - proper implementation would handle nested types
                    let elements: Vec<Option<String>> = arr
                        .iter()
                        .map(|v| match v {
                            PostgresValue::Null => None,
                            PostgresValue::Text(s) => Some(s.to_string()),
                            PostgresValue::Integer(i) => Some(i.to_string()),
                            _ => Some(format!("{:?}", v)),
                        })
                        .collect();
                    elements.to_sql(ty, out)
                }
            }
        }

        fn accepts(_ty: &Type) -> bool {
            // Accept all types - we'll handle conversion on a case-by-case basis
            true
        }

        // Use the appropriate macro based on which feature is enabled
        #[cfg(feature = "postgres-sync")]
        postgres::types::to_sql_checked!();

        #[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
        tokio_postgres::types::to_sql_checked!();
    }
}
