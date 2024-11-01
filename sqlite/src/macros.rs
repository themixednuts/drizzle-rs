// Main macro to construct the SQLiteColumn type with the appropriate generics
#[macro_export]
macro_rules! sqlite_column_type {
    (integer ($($params:tt)*) $(.$func:ident)*) => {
        $crate::prelude::SQLiteIntegerColumn<
            detect_integer_mode!($($params)*),
            detect_primary_key!($(.$func)*),
            detect_not_null!($(.$func)*),
            detect_unique!($(.$func)*),
            detect_autoincrement!($(.$func)*),
            detect_default!($(.$func)*),
            detect_default_fn!($(.$func)*),
        >
    };
    (real ($($params:tt)*) $(.$func:ident)*) => {
        $crate::prelude::SQLiteRealColumn<
            detect_real_mode!($($params)*),
            detect_primary_key!($(.$func)*),
            detect_not_null!($(.$func)*),
            detect_unique!($(.$func)*),
            detect_default!($(.$func)*),
            detect_default_fn!($(.$func)*),
        >
    };
    (text ($($params:tt)*) $(.$func:ident)*) => {
        $crate::prelude::SQLiteTextColumn<
            detect_text_mode!($($params)*),
            detect_primary_key!($(.$func)*),
            detect_not_null!($(.$func)*),
            detect_unique!($(.$func)*),
            detect_default!($(.$func)*),
            detect_default_fn!($(.$func)*),
        >
    };
    (blob ($($params:tt)*) $(.$func:ident)*) => {
        $crate::prelude::SQLiteBlobColumn<
            detect_primary_key!($(.$func)*),
            detect_not_null!($(.$func)*),
            detect_unique!($(.$func)*),
            detect_default!($(.$func)*),
            detect_default_fn!($(.$func)*),
        >
    };
}

// Detect specific function and assign corresponding type for PrimaryKey
#[macro_export]
macro_rules! detect_integer_mode {
    // Match pattern for SQLiteInteger with type parameters
    ($name:expr, SQLiteInteger { $($params:tt)*}) => {
        $crate::prelude::SQLiteInteger
    };
    ($name:expr, SQLiteTimeStamp { $($params:tt)*}) => {
        $crate::prelude::SQLiteTimeStamp
    };
    ($name:expr, SQLiteTimeStampMS { $($params:tt)*}) => {
        $crate::prelude::SQLiteTimeStampMS
    };
    ($name:expr, SQLiteBoolean { $($params:tt)*}) => {
        $crate::prelude::SQLiteBoolean
    };
    () => {
        ()
    };
}

// Detect specific function and assign corresponding type for PrimaryKey
#[macro_export]
macro_rules! detect_text_mode {
    // Match pattern for SQLiteInteger with type parameters
    ($name:expr, SQLiteText { $($params:tt)*}) => {
        $crate::prelude::SQLiteText
    };
    ($name:expr, SQLiteText::default()) => {
        $crate::prelude::SQLiteText
    };
    ($name:expr, SQLiteText $($params:tt)*) => {
        $crate::prelude::SQLiteText
    };
    ($name:expr, SQLiteTextEnum($($params:tt)*)) => {
        $crate::prelude::SQLiteTextEnum
    };
    ($name:expr, SQLiteJSON { $($params:tt)*}) => {
        $crate::prelude::SQLiteJSON
    };
    ($name:expr, SQLiteJSON::default()) => {
        $crate::prelude::SQLiteJSON
    };
    () => {
        ()
    };
}
// Detect specific function and assign corresponding type for PrimaryKey
#[macro_export]
macro_rules! detect_real_mode {
    // Match pattern for SQLiteInteger with type parameters
    ($name:expr, SQLiteReal { $($params:tt)*}) => {
        $crate::prelude::SQLiteReal
    };
    ($name:expr, SQLiteReal::default()) => {
        $crate::prelude::SQLiteReal
    };
    ($name:expr) => {
        $crate::prelude::SQLiteReal
    };
}

// Detect specific function and assign corresponding type for PrimaryKey
#[macro_export]
macro_rules! detect_primary_key {
    (.primary $(.$func:ident)*) => { $crate::prelude::IsPrimary };
    (.$head:ident $(.$func:ident)*) => {
        detect_primary_key!($(.$func)*)
    };
    () => { $crate::prelude::NotPrimary };
}

// Detect specific function and assign corresponding type for NotNull
#[macro_export]
macro_rules! detect_not_null {
    ($(.)?not_null $(.$func:ident)*) => {
        $crate::prelude::NotNullable
    };
    ($(.)?$head:ident $(.$func:ident)*) => {
        detect_not_null!($(.$func)*)
    };
    ($(.)?) => {
        $crate::prelude::Nullable
    };
}

// Detect specific function and assign corresponding type for Unique
#[macro_export]
macro_rules! detect_unique {
    (.unique $(.$func:ident)*) => {
        $crate::prelude::IsUnique
    };
    (.$head:ident $(.$func:ident)*) => {
        detect_unique!($(.$func)*)
    };
    ($(.)?) => {
        $crate::prelude::NotUnique
    };
}

// Detect specific function and assign corresponding type for Autoincremented
#[macro_export]
macro_rules! detect_autoincrement {
    (.autoincrement $(.$func:ident)*) => {
        $crate::prelude::IsAutoIncremented
    };
    (.$head:ident $(.$func:ident)*) => {
        detect_autoincrement!($(.$func)*)
    };
    () => {
        $crate::prelude::NotAutoIncremented
    };
}

// Detect specific function and assign corresponding type for Default
#[macro_export]
macro_rules! detect_default {
    () => {
        $crate::prelude::DefaultNotSet
    };
    (.default $(.$func:ident)*) => {
        $crate::prelude::DefaultSet
    };
    (.$head:ident $(.$func:ident)*) => {
        detect_default!($(.$func)*)
    };
}

// Detect specific function and assign corresponding type for DefaultFn
#[macro_export]
macro_rules! detect_default_fn {
    () => {
        $crate::prelude::DefaultFnNotSet
    };
    ($(.)?default_fn $(.$func:ident)*) => {
        $crate::prelude::DefaultFnSet
    };
    ($(.)?$head:ident $(.$func:ident)*) => {
        detect_default_fn!($(.$func)*)
    };
}

// Main macro to construct the SQLiteColumn type with the appropriate generics
#[macro_export]
macro_rules! sqlite_builder_to_column {
    (integer) => {
        $crate::prelude::SQLiteIntegerColumn
    };
    (real) => {
        $crate::prelude::SQLiteRealColumn
    };
    (text) => {
        $crate::prelude::SQLiteTextColumn
    };
    (blob) => {
        $crate::prelude::SQLiteBlobColumn
    };
    (any) => {
        $crate::prelude::SQLiteAnyColumn
    };
}

#[macro_export]
macro_rules! sqlite_table_internal {
    ($table_name:expr, { $($field_name:ident : $type:ident ( $($params:tt)* ) $(.$func:ident ( $($args:expr)* ))*),* $(,)? }) => {{

        $crate::prelude::paste! {

            #[derive(Clone, Debug)]
            pub struct [<$table_name:camel>] {
             $(
                #[allow(dead_code)]
                 pub $field_name: $crate::prelude::sqlite_column_type!($type( $($params)* ) $(.$func)*),
             )*
            }

            impl $crate::prelude::Table for [<$table_name:camel>] {
                type Schema = $crate::prelude::SQLiteTableSchema;
                fn name(&self) -> &'static str {
                    $table_name
                }

                fn schema(&self) -> Self::Schema {
                    Self::Schema { name: $table_name, _type: $crate::prelude::SQLiteTableType::Table }
                }
            }

            [<$table_name:camel>] {
                $($field_name: $type($($params)*)$(.$func($($args),*))*.into(),)*
            }
        }
    }};
}

#[macro_export]
macro_rules! sqlite_table {
    // Handle string literals
    ($table_name:literal, { $($rest:tt)* }) => {
        sqlite_table_internal!($table_name, { $($rest)* })
    };

    // // Handle identifiers
    // ($table_name:ident, { $($rest:tt)* }) => {
    //         sqlite_table!(@stringify_ident Vec::new([format!("{}", stringify!($table_name))]).join(", "), { $($rest)* })
    // };
    // // Handle identifiers
    // (@stringify_ident $table_name:ident, { $($rest:tt)* }) => {
    //     $crate::prelude::paste! {
    //         sqlite_table_internal!([<$table_name>], { $($rest)* })
    //     }
    // };
}

#[macro_export]
macro_rules! function_args {
    ($type:ident ( $($params:tt)* ) $( .$func:ident $( ( $($args:expr),* ) )+ )* ) => {
        $type( $( $params )* ) $( .$func $( ( $($args),* ) )+ ) *.into()
    };
    ($type:ident ( $($params:tt)* ) $( .$func:ident $( ( $($args:expr),* ) )? )* ) => {
        $type( $( $params )* ) $( .$func $( ( $($args),* ) )? ) *
    };
}

#[macro_export]
macro_rules! static_sqlite_table_internal {
    ($table_name:expr, { $($field_name:ident : $type:ident ( $($params:tt)* ) $(.$func:ident $( ( $($args:expr),* ) )? )* ),* $(,)? }) => {

        $crate::prelude::paste! {

            #[derive(Clone, Debug)]
            pub struct [<$table_name:camel>] {
             $(
                #[allow(dead_code)]
                 pub $field_name: $crate::prelude::sqlite_column_type!($type( $($params)* ) $(.$func)*),
             )*
            }

            impl $crate::prelude::Table for [<$table_name:camel>] {
                type Schema = $crate::prelude::SQLiteTableSchema;
                fn name(&self) -> &'static str {
                    $table_name
                }

                fn schema(&self) -> Self::Schema {
                    Self::Schema { name: $table_name, _type: $crate::prelude::SQLiteTableType::Table }
                }
            }

           pub static [<$table_name:snake:upper>]: ::std::sync::LazyLock<[<$table_name:camel>]> = ::std::sync::LazyLock::new(|| [<$table_name:camel>] {
                $($field_name: $crate::prelude::function_args!( $type ( $( $params )* ) $(.$func $( ( $($args),* ) )? )* )),*
            });
        }
    };

}

#[macro_export]
macro_rules! static_sqlite_table {
    // Handle string literals
    ($table_name:literal, { $($block:tt)* }) => {
        static_sqlite_table_internal!($table_name, { $( $block )* })
    }; // // Handle identifiers
       // ($table_name:ident, { $($rest:tt)* }) => {
       //         sqlite_table!(@stringify_ident Vec::new([format!("{}", stringify!($table_name))]).join(", "), { $($rest)* })
       // };
       // // Handle identifiers
       // (@stringify_ident $table_name:ident, { $($rest:tt)* }) => {
       //     $crate::prelude::paste! {
       //         sqlite_table_internal!([<$table_name>], { $($rest)* })
       //     }
       // };
}
