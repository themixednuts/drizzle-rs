//! Database driver implementations for `PostgresValue`

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use super::PostgresValue;

//------------------------------------------------------------------------------
// postgres/tokio-postgres ToSql implementations
// The two drivers expose the same ToSql contract, so one implementation covers both.
//------------------------------------------------------------------------------

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
mod postgres_tosql_impl {
    use super::PostgresValue;

    // Import from whichever crate is available
    #[cfg(feature = "postgres-sync")]
    use postgres::types::{IsNull, Kind, ToSql, Type};

    #[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
    use tokio_postgres::types::{IsNull, Kind, ToSql, Type};

    use bytes::BytesMut;

    macro_rules! encode_array {
        ($arr:expr, $ty:expr, $out:expr, $variant:ident, $rust_ty:ty, $convert:expr) => {{
            let mut values: Vec<Option<$rust_ty>> = Vec::with_capacity($arr.len());
            for value in $arr {
                match value {
                    PostgresValue::Null => values.push(None),
                    PostgresValue::$variant(inner) => values.push(Some($convert(inner))),
                    other => return Err(array_encode_error($ty, other)),
                }
            }
            values.to_sql($ty, $out)
        }};
    }

    fn array_encode_error(
        ty: &Type,
        value: &PostgresValue<'_>,
    ) -> Box<dyn std::error::Error + Sync + Send> {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "cannot encode {value:?} as an element of PostgreSQL array {}",
                ty.name()
            ),
        )
        .into()
    }

    fn array_to_sql(
        arr: &[PostgresValue<'_>],
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        if !matches!(ty.kind(), Kind::Array(_)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("cannot encode PostgreSQL array value as {}", ty.name()),
            )
            .into());
        }

        if *ty == Type::INT2_ARRAY {
            return encode_array!(arr, ty, out, Smallint, i16, |v: &i16| *v);
        }
        if *ty == Type::INT4_ARRAY {
            return encode_array!(arr, ty, out, Integer, i32, |v: &i32| *v);
        }
        if *ty == Type::INT8_ARRAY {
            return encode_array!(arr, ty, out, Bigint, i64, |v: &i64| *v);
        }
        if *ty == Type::FLOAT4_ARRAY {
            return encode_array!(arr, ty, out, Real, f32, |v: &f32| *v);
        }
        if *ty == Type::FLOAT8_ARRAY {
            return encode_array!(arr, ty, out, DoublePrecision, f64, |v: &f64| *v);
        }
        #[cfg(feature = "rust-decimal")]
        if *ty == Type::NUMERIC_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Numeric,
                rust_decimal::Decimal,
                |v: &rust_decimal::Decimal| *v
            );
        }
        if *ty == Type::TEXT_ARRAY || *ty == Type::VARCHAR_ARRAY || *ty == Type::BPCHAR_ARRAY {
            return encode_array!(arr, ty, out, Text, String, |v: &std::borrow::Cow<
                '_,
                str,
            >| v.to_string());
        }
        if *ty == Type::BYTEA_ARRAY {
            return encode_array!(arr, ty, out, Bytea, Vec<u8>, |v: &std::borrow::Cow<
                '_,
                [u8],
            >| v.to_vec());
        }
        if *ty == Type::BOOL_ARRAY {
            return encode_array!(arr, ty, out, Boolean, bool, |v: &bool| *v);
        }
        #[cfg(feature = "uuid")]
        if *ty == Type::UUID_ARRAY {
            return encode_array!(arr, ty, out, Uuid, uuid::Uuid, |v: &uuid::Uuid| *v);
        }
        #[cfg(feature = "serde")]
        if *ty == Type::JSON_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Json,
                serde_json::Value,
                |v: &serde_json::Value| v.clone()
            );
        }
        #[cfg(feature = "serde")]
        if *ty == Type::JSONB_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Jsonb,
                serde_json::Value,
                |v: &serde_json::Value| v.clone()
            );
        }
        #[cfg(feature = "chrono")]
        if *ty == Type::DATE_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Date,
                chrono::NaiveDate,
                |v: &chrono::NaiveDate| *v
            );
        }
        #[cfg(feature = "chrono")]
        if *ty == Type::TIME_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Time,
                chrono::NaiveTime,
                |v: &chrono::NaiveTime| *v
            );
        }
        #[cfg(feature = "chrono")]
        if *ty == Type::TIMESTAMP_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                Timestamp,
                chrono::NaiveDateTime,
                |v: &chrono::NaiveDateTime| *v
            );
        }
        #[cfg(feature = "chrono")]
        if *ty == Type::TIMESTAMPTZ_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                TimestampTz,
                chrono::DateTime<chrono::FixedOffset>,
                |v: &chrono::DateTime<chrono::FixedOffset>| *v
            );
        }
        #[cfg(feature = "time")]
        if *ty == Type::DATE_ARRAY {
            return encode_array!(arr, ty, out, TimeDate, time::Date, |v: &time::Date| *v);
        }
        #[cfg(feature = "time")]
        if *ty == Type::TIME_ARRAY {
            return encode_array!(arr, ty, out, TimeTime, time::Time, |v: &time::Time| *v);
        }
        #[cfg(feature = "time")]
        if *ty == Type::TIMESTAMP_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                TimeTimestamp,
                time::PrimitiveDateTime,
                |v: &time::PrimitiveDateTime| *v
            );
        }
        #[cfg(feature = "time")]
        if *ty == Type::TIMESTAMPTZ_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                TimeTimestampTz,
                time::OffsetDateTime,
                |v: &time::OffsetDateTime| *v
            );
        }
        #[cfg(feature = "cidr")]
        if *ty == Type::INET_ARRAY {
            return encode_array!(arr, ty, out, Inet, cidr::IpInet, |v: &cidr::IpInet| *v);
        }
        #[cfg(feature = "cidr")]
        if *ty == Type::CIDR_ARRAY {
            return encode_array!(arr, ty, out, Cidr, cidr::IpCidr, |v: &cidr::IpCidr| *v);
        }
        #[cfg(feature = "bit-vec")]
        if *ty == Type::VARBIT_ARRAY || *ty == Type::BIT_ARRAY {
            return encode_array!(
                arr,
                ty,
                out,
                BitVec,
                bit_vec::BitVec,
                |v: &bit_vec::BitVec| v.clone()
            );
        }

        let mut values: Vec<Option<String>> = Vec::with_capacity(arr.len());
        for value in arr {
            match value {
                PostgresValue::Null => values.push(None),
                PostgresValue::Text(value) => values.push(Some(value.to_string())),
                PostgresValue::Enum(value) => values.push(Some(value.variant_name().to_owned())),
                other => return Err(array_encode_error(ty, other)),
            }
        }
        values.to_sql(ty, out)
    }

    impl ToSql for PostgresValue<'_> {
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
                #[cfg(feature = "time")]
                PostgresValue::TimeDate(date) => date.to_sql(ty, out),
                #[cfg(feature = "time")]
                PostgresValue::TimeTime(time) => time.to_sql(ty, out),
                #[cfg(feature = "time")]
                PostgresValue::TimeTimestamp(ts) => ts.to_sql(ty, out),
                #[cfg(feature = "time")]
                PostgresValue::TimeTimestampTz(ts) => ts.to_sql(ty, out),
                #[cfg(feature = "time")]
                PostgresValue::TimeInterval(dur) => dur.to_string().to_sql(ty, out),
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
                PostgresValue::Array(arr) => array_to_sql(arr, ty, out),
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
