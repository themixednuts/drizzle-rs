use super::DataType;

#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` is not compatible with `{Rhs}`",
    label = "these SQL types cannot be compared or coerced",
    note = "compatible types include: integers with integers/floats, text with text/varchar, and any type with itself"
)]
pub trait Compatible<Rhs: DataType = Self>: DataType {}

#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` is not assignable from `{Rhs}`",
    label = "this value type cannot be bound to the placeholder/column type",
    note = "assignment is stricter than comparison compatibility; use a value matching the target SQL marker"
)]
pub trait Assignable<Rhs: DataType = Self>: DataType {}

// =============================================================================
// Core macros
// =============================================================================

macro_rules! impl_reflexive_compat {
    ($($ty:ty),+ $(,)?) => {
        $(impl Compatible<$ty> for $ty {})+
    };
}

macro_rules! impl_reflexive_assign {
    ($($ty:ty),+ $(,)?) => {
        $(impl Assignable<$ty> for $ty {})+
    };
}

/// Generate mutual (bidirectional) `Compatible` impls for all unique cross-pairs in a group.
/// Does NOT generate reflexive impls (those come from `impl_reflexive_compat!`).
///
/// `mutual_compat!(A, B, C)` expands to:
///   Compatible<B> for A, Compatible<A> for B,
///   Compatible<C> for A, Compatible<A> for C,
///   Compatible<C> for B, Compatible<B> for C
macro_rules! mutual_compat {
    // Entry: dispatch to pair generation
    ($($ty:ty),+ $(,)?) => {
        mutual_compat!(@pairs [] $($ty),+);
    };
    // Base case: no more types to process
    (@pairs [$($done:ty),*]) => {};
    // Recursive case: take first type, pair it with all remaining
    (@pairs [$($done:ty),*] $first:ty $(, $rest:ty)*) => {
        $(
            impl Compatible<$rest> for $first {}
            impl Compatible<$first> for $rest {}
        )*
        mutual_compat!(@pairs [$($done,)* $first] $($rest),*);
    };
}

/// Generate bidirectional `Compatible` impls between an "any" type and a list of types.
macro_rules! any_compat {
    ($any:ty; $($ty:ty),+ $(,)?) => {
        $(
            impl Compatible<$ty> for $any {}
            impl Compatible<$any> for $ty {}
        )+
    };
}

macro_rules! impl_placeholder_compat {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Compatible<crate::Placeholder> for $ty {}
            impl Compatible<$ty> for crate::Placeholder {}
        )+
    };
}

/// Generate directional `Assignable` impls: each source can be assigned to the target.
/// `assign_to!(Target; Src1, Src2)` → `Assignable<Src1> for Target`, `Assignable<Src2> for Target`
macro_rules! assign_to {
    ($target:ty; $($src:ty),+ $(,)?) => {
        $(impl Assignable<$src> for $target {})+
    };
}

/// Generate bidirectional `Assignable` for all cross-pairs in a group (mutual assignment).
/// Does NOT generate reflexive impls (those come from `impl_reflexive_assign!`).
macro_rules! mutual_assign {
    ($($ty:ty),+ $(,)?) => {
        mutual_assign!(@pairs [] $($ty),+);
    };
    (@pairs [$($done:ty),*]) => {};
    (@pairs [$($done:ty),*] $first:ty $(, $rest:ty)*) => {
        $(
            impl Assignable<$rest> for $first {}
            impl Assignable<$first> for $rest {}
        )*
        mutual_assign!(@pairs [$($done,)* $first] $($rest),*);
    };
}

/// Generate `Assignable` from every source to a single "any" target.
macro_rules! any_assign {
    ($any:ty; $($ty:ty),+ $(,)?) => {
        $(impl Assignable<$ty> for $any {})+
    };
}

// =============================================================================
// Generic impls
// =============================================================================

impl<T: DataType> Compatible<crate::Array<T>> for crate::Array<T> {}
impl Compatible<crate::Placeholder> for crate::Placeholder {}
impl<T: DataType> Assignable<crate::Array<T>> for crate::Array<T> {}

// Reflexive compat & assign for all concrete types
impl_reflexive_compat!(
    crate::sqlite::types::Integer,
    crate::sqlite::types::Text,
    crate::sqlite::types::Real,
    crate::sqlite::types::Blob,
    crate::sqlite::types::Numeric,
    crate::sqlite::types::Any,
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8,
    crate::postgres::types::Varchar,
    crate::postgres::types::Text,
    crate::postgres::types::Char,
    crate::postgres::types::Bytea,
    crate::postgres::types::Boolean,
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp,
    crate::postgres::types::Date,
    crate::postgres::types::Time,
    crate::postgres::types::Timetz,
    crate::postgres::types::Numeric,
    crate::postgres::types::Uuid,
    crate::postgres::types::Json,
    crate::postgres::types::Jsonb,
    crate::postgres::types::Any,
    crate::postgres::types::Interval,
    crate::postgres::types::Inet,
    crate::postgres::types::Cidr,
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8,
    crate::postgres::types::Point,
    crate::postgres::types::LineString,
    crate::postgres::types::Rect,
    crate::postgres::types::BitString,
    crate::postgres::types::Line,
    crate::postgres::types::LineSegment,
    crate::postgres::types::Polygon,
    crate::postgres::types::Circle,
    crate::postgres::types::Enum,
);

impl_reflexive_assign!(
    crate::sqlite::types::Integer,
    crate::sqlite::types::Text,
    crate::sqlite::types::Real,
    crate::sqlite::types::Blob,
    crate::sqlite::types::Numeric,
    crate::sqlite::types::Any,
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8,
    crate::postgres::types::Varchar,
    crate::postgres::types::Text,
    crate::postgres::types::Char,
    crate::postgres::types::Bytea,
    crate::postgres::types::Boolean,
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp,
    crate::postgres::types::Date,
    crate::postgres::types::Time,
    crate::postgres::types::Timetz,
    crate::postgres::types::Numeric,
    crate::postgres::types::Uuid,
    crate::postgres::types::Json,
    crate::postgres::types::Jsonb,
    crate::postgres::types::Any,
    crate::postgres::types::Interval,
    crate::postgres::types::Inet,
    crate::postgres::types::Cidr,
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8,
    crate::postgres::types::Point,
    crate::postgres::types::LineString,
    crate::postgres::types::Rect,
    crate::postgres::types::BitString,
    crate::postgres::types::Line,
    crate::postgres::types::LineSegment,
    crate::postgres::types::Polygon,
    crate::postgres::types::Circle,
    crate::postgres::types::Enum,
);

// =============================================================================
// SQLite compatibility (category-based)
// =============================================================================

// SQLite numeric family: Integer ↔ Real ↔ Numeric (all mutually compatible)
mutual_compat!(
    crate::sqlite::types::Integer,
    crate::sqlite::types::Real,
    crate::sqlite::types::Numeric
);

// Blob ↔ Text (SQLite stores UUIDs, etc. as either)
mutual_compat!(crate::sqlite::types::Text, crate::sqlite::types::Blob);

// Any ↔ all SQLite types
any_compat!(crate::sqlite::types::Any;
    crate::sqlite::types::Integer,
    crate::sqlite::types::Text,
    crate::sqlite::types::Real,
    crate::sqlite::types::Blob,
    crate::sqlite::types::Numeric
);

impl_placeholder_compat!(
    crate::sqlite::types::Integer,
    crate::sqlite::types::Text,
    crate::sqlite::types::Real,
    crate::sqlite::types::Blob,
    crate::sqlite::types::Numeric,
    crate::sqlite::types::Any
);

// =============================================================================
// PostgreSQL compatibility (category-based)
// =============================================================================

// Integer family: Int2 ↔ Int4 ↔ Int8 (all mutually compatible)
mutual_compat!(
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8
);

// Float family: Float4 ↔ Float8
mutual_compat!(
    crate::postgres::types::Float4,
    crate::postgres::types::Float8
);

// Cross-numeric: all integers ↔ all floats
// (Int2, Int4, Int8) × (Float4, Float8)
/// Generate cross-product `Compatible` impls between two groups of types.
/// Every type in group A becomes compatible with every type in group B (bidirectional).
macro_rules! cross_compat {
    ([$first_a:ty $(, $rest_a:ty)* $(,)?], [$($b:ty),+ $(,)?]) => {
        // Expand first_a × all B
        $(
            impl Compatible<$b> for $first_a {}
            impl Compatible<$first_a> for $b {}
        )+
        // Recurse for remaining A types
        cross_compat!([$($rest_a),*], [$($b),+]);
    };
    // Base case: empty A list
    ([], [$($b:ty),+ $(,)?]) => {};
}

cross_compat!(
    [
        crate::postgres::types::Int2,
        crate::postgres::types::Int4,
        crate::postgres::types::Int8
    ],
    [
        crate::postgres::types::Float4,
        crate::postgres::types::Float8
    ]
);

// Numeric ↔ all numeric types (Int2, Int4, Int8, Float4, Float8)
any_compat!(crate::postgres::types::Numeric;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8
);

// Text family: Text ↔ Varchar ↔ Char (all mutually compatible)
mutual_compat!(
    crate::postgres::types::Text,
    crate::postgres::types::Varchar,
    crate::postgres::types::Char
);

// Temporal cross-compatibility pairs
mutual_compat!(
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp
);
mutual_compat!(crate::postgres::types::Time, crate::postgres::types::Timetz);

// JSON cross-compatibility
mutual_compat!(crate::postgres::types::Json, crate::postgres::types::Jsonb);

// Text ↔ Temporal (string comparisons with timestamps)
cross_compat!(
    [crate::postgres::types::Text],
    [
        crate::postgres::types::Timestamptz,
        crate::postgres::types::Timestamp,
        crate::postgres::types::Date,
        crate::postgres::types::Time
    ]
);

// Network types: Inet ↔ Cidr
mutual_compat!(crate::postgres::types::Inet, crate::postgres::types::Cidr);

// MAC address types: MacAddr ↔ MacAddr8
mutual_compat!(
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8
);

// Enum ↔ Text family
cross_compat!(
    [crate::postgres::types::Enum],
    [
        crate::postgres::types::Text,
        crate::postgres::types::Varchar,
        crate::postgres::types::Char
    ]
);

// Any ↔ all PostgreSQL types
any_compat!(crate::postgres::types::Any;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8,
    crate::postgres::types::Varchar,
    crate::postgres::types::Text,
    crate::postgres::types::Char,
    crate::postgres::types::Bytea,
    crate::postgres::types::Boolean,
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp,
    crate::postgres::types::Date,
    crate::postgres::types::Time,
    crate::postgres::types::Timetz,
    crate::postgres::types::Numeric,
    crate::postgres::types::Uuid,
    crate::postgres::types::Json,
    crate::postgres::types::Jsonb,
    crate::postgres::types::Interval,
    crate::postgres::types::Inet,
    crate::postgres::types::Cidr,
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8,
    crate::postgres::types::Point,
    crate::postgres::types::LineString,
    crate::postgres::types::Rect,
    crate::postgres::types::BitString,
    crate::postgres::types::Line,
    crate::postgres::types::LineSegment,
    crate::postgres::types::Polygon,
    crate::postgres::types::Circle,
    crate::postgres::types::Enum
);

impl_placeholder_compat!(
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8,
    crate::postgres::types::Varchar,
    crate::postgres::types::Text,
    crate::postgres::types::Char,
    crate::postgres::types::Bytea,
    crate::postgres::types::Boolean,
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp,
    crate::postgres::types::Date,
    crate::postgres::types::Time,
    crate::postgres::types::Timetz,
    crate::postgres::types::Numeric,
    crate::postgres::types::Uuid,
    crate::postgres::types::Json,
    crate::postgres::types::Jsonb,
    crate::postgres::types::Any,
    crate::postgres::types::Interval,
    crate::postgres::types::Inet,
    crate::postgres::types::Cidr,
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8,
    crate::postgres::types::Point,
    crate::postgres::types::LineString,
    crate::postgres::types::Rect,
    crate::postgres::types::BitString,
    crate::postgres::types::Line,
    crate::postgres::types::LineSegment,
    crate::postgres::types::Polygon,
    crate::postgres::types::Circle,
    crate::postgres::types::Enum
);

// =============================================================================
// Assignment compatibility (bind-time)
// =============================================================================
// Assignment is stricter than comparison: only wider types accept narrower values.

// SQLite assignment
assign_to!(crate::sqlite::types::Real; crate::sqlite::types::Integer);
assign_to!(crate::sqlite::types::Numeric; crate::sqlite::types::Integer, crate::sqlite::types::Real);
any_assign!(crate::sqlite::types::Any;
    crate::sqlite::types::Integer,
    crate::sqlite::types::Text,
    crate::sqlite::types::Real,
    crate::sqlite::types::Blob,
    crate::sqlite::types::Numeric
);

// PostgreSQL integer widening: Int2 → Int4 → Int8
assign_to!(crate::postgres::types::Int4; crate::postgres::types::Int2);
assign_to!(crate::postgres::types::Int8; crate::postgres::types::Int2, crate::postgres::types::Int4);

// Int → Float widening
assign_to!(crate::postgres::types::Float4;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8
);
assign_to!(crate::postgres::types::Float8;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4
);

// Numeric accepts all numeric types
assign_to!(crate::postgres::types::Numeric;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8
);

// Text family: all mutually assignable
mutual_assign!(
    crate::postgres::types::Text,
    crate::postgres::types::Varchar,
    crate::postgres::types::Char
);

// Enum → Text family
assign_to!(crate::postgres::types::Varchar; crate::postgres::types::Enum);
assign_to!(crate::postgres::types::Text; crate::postgres::types::Enum);
assign_to!(crate::postgres::types::Char; crate::postgres::types::Enum);

// Temporal and JSON assignment
assign_to!(crate::postgres::types::Timestamptz; crate::postgres::types::Timestamp);
assign_to!(crate::postgres::types::Timetz; crate::postgres::types::Time);
assign_to!(crate::postgres::types::Jsonb; crate::postgres::types::Json);

// Network assignment
assign_to!(crate::postgres::types::Cidr; crate::postgres::types::Inet);

// Any accepts all concrete PostgreSQL markers
any_assign!(crate::postgres::types::Any;
    crate::postgres::types::Int2,
    crate::postgres::types::Int4,
    crate::postgres::types::Int8,
    crate::postgres::types::Float4,
    crate::postgres::types::Float8,
    crate::postgres::types::Varchar,
    crate::postgres::types::Text,
    crate::postgres::types::Char,
    crate::postgres::types::Bytea,
    crate::postgres::types::Boolean,
    crate::postgres::types::Timestamptz,
    crate::postgres::types::Timestamp,
    crate::postgres::types::Date,
    crate::postgres::types::Time,
    crate::postgres::types::Timetz,
    crate::postgres::types::Numeric,
    crate::postgres::types::Uuid,
    crate::postgres::types::Json,
    crate::postgres::types::Jsonb,
    crate::postgres::types::Interval,
    crate::postgres::types::Inet,
    crate::postgres::types::Cidr,
    crate::postgres::types::MacAddr,
    crate::postgres::types::MacAddr8,
    crate::postgres::types::Point,
    crate::postgres::types::LineString,
    crate::postgres::types::Rect,
    crate::postgres::types::BitString,
    crate::postgres::types::Line,
    crate::postgres::types::LineSegment,
    crate::postgres::types::Polygon,
    crate::postgres::types::Circle,
    crate::postgres::types::Enum
);

// =============================================================================
// Tuple compatibility
// =============================================================================

macro_rules! seq_dual {
    (@acc $callback:ident [$($sa:ident),*] [$($da:ident),*]) => {};
    (@acc $callback:ident [$($sa:ident),*] [$($da:ident),*] ($s:ident, $d:ident) $($rest:tt)*) => {
        $callback!($($sa,)* $s; $($da,)* $d);
        seq_dual!(@acc $callback [$($sa,)* $s] [$($da,)* $d] $($rest)*);
    };
    ($callback:ident; $($pairs:tt)+) => {
        seq_dual!(@acc $callback [] [] $($pairs)+);
    };
    (@from $callback:ident [$($sa:ident),*] [$($da:ident),*]; $($pairs:tt)+) => {
        seq_dual!(@acc $callback [$($sa),*] [$($da),*] $($pairs)+);
    };
}

macro_rules! with_dual_col_sizes_8 {
    ($callback:ident) => {
        seq_dual!($callback;
            (T0,U0) (T1,U1) (T2,U2) (T3,U3)
            (T4,U4) (T5,U5) (T6,U6) (T7,U7)
        );
    };
}

#[allow(unused_macros)]
macro_rules! with_dual_col_sizes_16 {
    ($callback:ident) => {
        seq_dual!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7]
            [U0,U1,U2,U3,U4,U5,U6,U7];
            (T8,U8) (T9,U9) (T10,U10) (T11,U11)
            (T12,U12) (T13,U13) (T14,U14) (T15,U15)
        );
    };
}

#[allow(unused_macros)]
macro_rules! with_dual_col_sizes_32 {
    ($callback:ident) => {
        seq_dual!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15]
            [U0,U1,U2,U3,U4,U5,U6,U7,U8,U9,U10,U11,U12,U13,U14,U15];
            (T16,U16) (T17,U17) (T18,U18) (T19,U19)
            (T20,U20) (T21,U21) (T22,U22) (T23,U23)
            (T24,U24) (T25,U25) (T26,U26) (T27,U27)
            (T28,U28) (T29,U29) (T30,U30) (T31,U31)
        );
    };
}

#[allow(unused_macros)]
macro_rules! with_dual_col_sizes_64 {
    ($callback:ident) => {
        seq_dual!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31]
            [U0,U1,U2,U3,U4,U5,U6,U7,U8,U9,U10,U11,U12,U13,U14,U15,
             U16,U17,U18,U19,U20,U21,U22,U23,U24,U25,U26,U27,U28,U29,U30,U31];
            (T32,U32) (T33,U33) (T34,U34) (T35,U35)
            (T36,U36) (T37,U37) (T38,U38) (T39,U39)
            (T40,U40) (T41,U41) (T42,U42) (T43,U43)
            (T44,U44) (T45,U45) (T46,U46) (T47,U47)
            (T48,U48) (T49,U49) (T50,U50) (T51,U51)
            (T52,U52) (T53,U53) (T54,U54) (T55,U55)
            (T56,U56) (T57,U57) (T58,U58) (T59,U59)
            (T60,U60) (T61,U61) (T62,U62) (T63,U63)
        );
    };
}

#[allow(unused_macros)]
macro_rules! with_dual_col_sizes_128 {
    ($callback:ident) => {
        seq_dual!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31,
             T32,T33,T34,T35,T36,T37,T38,T39,T40,T41,T42,T43,T44,T45,T46,T47,
             T48,T49,T50,T51,T52,T53,T54,T55,T56,T57,T58,T59,T60,T61,T62,T63]
            [U0,U1,U2,U3,U4,U5,U6,U7,U8,U9,U10,U11,U12,U13,U14,U15,
             U16,U17,U18,U19,U20,U21,U22,U23,U24,U25,U26,U27,U28,U29,U30,U31,
             U32,U33,U34,U35,U36,U37,U38,U39,U40,U41,U42,U43,U44,U45,U46,U47,
             U48,U49,U50,U51,U52,U53,U54,U55,U56,U57,U58,U59,U60,U61,U62,U63];
            (T64,U64) (T65,U65) (T66,U66) (T67,U67)
            (T68,U68) (T69,U69) (T70,U70) (T71,U71)
            (T72,U72) (T73,U73) (T74,U74) (T75,U75)
            (T76,U76) (T77,U77) (T78,U78) (T79,U79)
            (T80,U80) (T81,U81) (T82,U82) (T83,U83)
            (T84,U84) (T85,U85) (T86,U86) (T87,U87)
            (T88,U88) (T89,U89) (T90,U90) (T91,U91)
            (T92,U92) (T93,U93) (T94,U94) (T95,U95)
            (T96,U96) (T97,U97) (T98,U98) (T99,U99)
            (T100,U100) (T101,U101) (T102,U102) (T103,U103)
            (T104,U104) (T105,U105) (T106,U106) (T107,U107)
            (T108,U108) (T109,U109) (T110,U110) (T111,U111)
            (T112,U112) (T113,U113) (T114,U114) (T115,U115)
            (T116,U116) (T117,U117) (T118,U118) (T119,U119)
            (T120,U120) (T121,U121) (T122,U122) (T123,U123)
            (T124,U124) (T125,U125) (T126,U126) (T127,U127)
        );
    };
}

#[allow(unused_macros)]
macro_rules! with_dual_col_sizes_200 {
    ($callback:ident) => {
        seq_dual!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31,
             T32,T33,T34,T35,T36,T37,T38,T39,T40,T41,T42,T43,T44,T45,T46,T47,
             T48,T49,T50,T51,T52,T53,T54,T55,T56,T57,T58,T59,T60,T61,T62,T63,
             T64,T65,T66,T67,T68,T69,T70,T71,T72,T73,T74,T75,T76,T77,T78,T79,
             T80,T81,T82,T83,T84,T85,T86,T87,T88,T89,T90,T91,T92,T93,T94,T95,
             T96,T97,T98,T99,T100,T101,T102,T103,T104,T105,T106,T107,T108,T109,T110,T111,
             T112,T113,T114,T115,T116,T117,T118,T119,T120,T121,T122,T123,T124,T125,T126,T127]
            [U0,U1,U2,U3,U4,U5,U6,U7,U8,U9,U10,U11,U12,U13,U14,U15,
             U16,U17,U18,U19,U20,U21,U22,U23,U24,U25,U26,U27,U28,U29,U30,U31,
             U32,U33,U34,U35,U36,U37,U38,U39,U40,U41,U42,U43,U44,U45,U46,U47,
             U48,U49,U50,U51,U52,U53,U54,U55,U56,U57,U58,U59,U60,U61,U62,U63,
             U64,U65,U66,U67,U68,U69,U70,U71,U72,U73,U74,U75,U76,U77,U78,U79,
             U80,U81,U82,U83,U84,U85,U86,U87,U88,U89,U90,U91,U92,U93,U94,U95,
             U96,U97,U98,U99,U100,U101,U102,U103,U104,U105,U106,U107,U108,U109,U110,U111,
             U112,U113,U114,U115,U116,U117,U118,U119,U120,U121,U122,U123,U124,U125,U126,U127];
            (T128,U128) (T129,U129) (T130,U130) (T131,U131)
            (T132,U132) (T133,U133) (T134,U134) (T135,U135)
            (T136,U136) (T137,U137) (T138,U138) (T139,U139)
            (T140,U140) (T141,U141) (T142,U142) (T143,U143)
            (T144,U144) (T145,U145) (T146,U146) (T147,U147)
            (T148,U148) (T149,U149) (T150,U150) (T151,U151)
            (T152,U152) (T153,U153) (T154,U154) (T155,U155)
            (T156,U156) (T157,U157) (T158,U158) (T159,U159)
            (T160,U160) (T161,U161) (T162,U162) (T163,U163)
            (T164,U164) (T165,U165) (T166,U166) (T167,U167)
            (T168,U168) (T169,U169) (T170,U170) (T171,U171)
            (T172,U172) (T173,U173) (T174,U174) (T175,U175)
            (T176,U176) (T177,U177) (T178,U178) (T179,U179)
            (T180,U180) (T181,U181) (T182,U182) (T183,U183)
            (T184,U184) (T185,U185) (T186,U186) (T187,U187)
            (T188,U188) (T189,U189) (T190,U190) (T191,U191)
            (T192,U192) (T193,U193) (T194,U194) (T195,U195)
            (T196,U196) (T197,U197) (T198,U198) (T199,U199)
        );
    };
}

macro_rules! impl_tuple_compatible {
    ($($T:ident),+; $($U:ident),+) => {
        impl<$($T, $U),+> Compatible<($($U,)+)> for ($($T,)+)
        where
            $($T: DataType + Compatible<$U>, $U: DataType,)+
        {}
    };
}

macro_rules! impl_tuple_assignable {
    ($($S:ident),+; $($D:ident),+) => {
        impl<$($S, $D),+> Assignable<($($S,)+)> for ($($D,)+)
        where
            $($S: DataType, $D: DataType + Assignable<$S>,)+
        {}
    };
}

with_dual_col_sizes_8!(impl_tuple_compatible);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_dual_col_sizes_16!(impl_tuple_compatible);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_dual_col_sizes_32!(impl_tuple_compatible);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_dual_col_sizes_64!(impl_tuple_compatible);

#[cfg(any(feature = "col128", feature = "col200"))]
with_dual_col_sizes_128!(impl_tuple_compatible);

#[cfg(feature = "col200")]
with_dual_col_sizes_200!(impl_tuple_compatible);

with_dual_col_sizes_8!(impl_tuple_assignable);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_dual_col_sizes_16!(impl_tuple_assignable);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_dual_col_sizes_32!(impl_tuple_assignable);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_dual_col_sizes_64!(impl_tuple_assignable);

#[cfg(any(feature = "col128", feature = "col200"))]
with_dual_col_sizes_128!(impl_tuple_assignable);

#[cfg(feature = "col200")]
with_dual_col_sizes_200!(impl_tuple_assignable);
