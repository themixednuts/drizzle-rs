extern crate common;
// #[cfg(feature = "sqlite")]
extern crate sqlite;

pub mod prelude {
    pub use common::*;
    // #[cfg(feature = "sqlite")]
    pub use sqlite::prelude::*;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn create_table() {
        static_sqlite_table!("users_table", {
            id: integer("id", SQLiteInteger {}).primary().not_null(),
            name: text("name", SQLiteText {}),
        });

        println!("{}", USERS_TABLE.name.to_sql());
    }
}
