pub mod prelude {
    pub use procmacros::*;
    pub use querybuilder::core::*;
    pub use querybuilder::sqlite::prelude::*;
    pub use querybuilder::*;
}

#[cfg(test)]
mod tests {

    use super::prelude::*;
    use uuid::Uuid;

    #[allow(dead_code)]
    #[derive(SQLiteTable)]
    #[table(strict)]
    pub struct Users {
        #[text(primary_key)]
        id: String,
        #[text]
        name: String,
        #[text]
        email: Option<String>,
        #[integer]
        is_active: i64,
    }

    #[test]
    fn test_table_name() {
        let sql = Users::id;
        println!("{}", sql);

        let placeholder = placeholder!("id");
        println!("{}", placeholder);
    }

    #[allow(dead_code)]
    #[derive(SQLiteTable)]
    #[table(strict)]
    struct Follow {
        #[integer(primary_key)]
        id: i64,
    }
}
