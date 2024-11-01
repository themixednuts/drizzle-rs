mod columns;
mod common;
mod macros;
mod table;
mod traits;

pub mod prelude {
    pub use crate::{
        columns::{
            blob::{blob, SQLiteBlob, SQLiteBlobColumn, SQLiteBlobColumnBuilder},
            integer::{
                integer, IsAutoIncremented, NotAutoIncremented, SQLiteBoolean, SQLiteInteger,
                SQLiteIntegerColumn, SQLiteIntegerColumnBuilder, SQLiteIntegerMode,
                SQLiteTimeStamp, SQLiteTimeStampMS,
            },
            real::{real, SQLiteReal, SQLiteRealColumn, SQLiteRealColumnBuilder, SQLiteRealMode},
            text::{
                text, SQLiteJSON, SQLiteText, SQLiteTextColumn, SQLiteTextColumnBuilder,
                SQLiteTextEnum, SQLiteTextMode,
            },
            DefaultFnNotSet, DefaultFnSet, DefaultNotSet, DefaultSet, IsPrimary, IsUnique,
            NotNullable, NotPrimary, NotUnique, Nullable,
        },
        common::{SQLiteTableSchema, SQLiteTableType},
        detect_autoincrement, detect_default, detect_default_fn, detect_integer_mode,
        detect_not_null, detect_primary_key, detect_real_mode, detect_text_mode, detect_unique,
        function_args, sqlite_builder_to_column, sqlite_column_type, sqlite_table,
        sqlite_table_internal, static_sqlite_table, static_sqlite_table_internal,
        traits::column::{Autoincrement, SQLAutoIncrement},
    };
    pub use common::traits::*;
    pub use paste::paste;
}

#[cfg(test)]
mod tests {

    use super::prelude::*;
    use common::ToSQL;

    #[test]
    fn table() {
        // let table = SQLiteTable {}
        //     .column(integer("id", SQLiteIntegerMode::Number))
        //     .column(real("value"));

        // macro_rules! sql {
        //     // Capture the format string and a list of expressions
        //     ($($arg:tt)*) => {{
        //         // Use format_args! to generate a formatted string
        //         let query = format!($($arg)*);
        //         // SQL { query }
        //         query
        //     }};
        // }

        let id = integer("id", SQLiteInteger::default())
            .primary()
            .not_null()
            .default_fn(|| Ok(42));

        let id = SQLiteIntegerColumn::from(id);
        id.to_sql();

        // let name = text("name", Default::default())
        //     .primary()
        //     .not_null()
        //     .default("".into());
        // let value = text("name", Default::default()).primary();

        // let t = sqlite_table("users_table")
        //     .strict()
        //     .without_rowid()
        //     .add_column(id)
        //     .add_column(name)
        //     .add_column(value)
        //     .finalize();

        // let sql = sql!("SELECT * FROM users WHERE id = {}", table.id);

        // let users = sqlite_table!("users_table", {
        //     id: integer("id", SQLiteInteger {}).primary().not_null().default_fn(|| {
        //         Ok(42)
        //     }),
        //     cost: real("cost").not_null(),
        //     name: text("name", Default::default()).primary().not_null().default("".into()),
        //     value: text("name", Default::default()).primary(),
        //     buffer: blob("buffer"),
        // });

        // users.name;
    }
}
