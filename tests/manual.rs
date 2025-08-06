// use drizzle_core::{SQLColumn, SQLSchema, SQLSchemaType, SQLTable};
// use drizzle_rs::core::{SQL, SQLModel, ToSQL};
// use drizzle_rs::sqlite::SQLiteValue;
// use rusqlite::Connection;

// // Manual implementation of SQLTable trait
// #[derive(Debug)]
// struct User {
//     id: i32,
//     name: String,
//     email: Option<String>,
// }
// impl<'a> SQLSchema<'a, SQLSchemaType> for User {
//     const NAME: &'static str = "manual_users";
//     const TYPE: SQLSchemaType = SQLSchemaType::Table;
//     const SQL: &'a str = "CREATE TABLE";
// }

// impl<'a> SQLTable<'a, SQLiteValue<'a>> for User {
//     type Schema = Self;
//     type Insert = InsertUser;
//     type Select;
//     type Update;
//     type Columns = (SQLColumn<'a, SQLiteValue<'a>>);

//     const COUNT: usize;
//     const COLUMNS: Self::Columns;
// }

// #[derive(Debug, Default)]
// struct InsertUser {
//     name: String,
//     email: Option<String>,
// }

// impl<'a> SQLModel<'a, SQLiteValue<'a>> for InsertUser {
//     fn columns(&self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
//         todo!()
//     }

//     fn values(&self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
//         todo!()
//     }
// }

// // Manual ToSQL implementation for insert data
// impl<'a> ToSQL<'a, SQLiteValue<'a>> for InsertUser {
//     fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
//         let mut sql = SQL::raw("(");
//         sql = sql.append(SQL::parameter(SQLiteValue::Text(self.name.clone().into())));
//         sql = sql.append_raw(", ");

//         match &self.email {
//             Some(email) => {
//                 sql = sql.append(SQL::parameter(SQLiteValue::Text(email.clone().into())));
//             }
//             None => {
//                 sql = sql.append(SQL::parameter(SQLiteValue::Null));
//             }
//         }

//         sql.append_raw(")")
//     }
// }

// #[test]
// fn manual_sql_building() {
//     let sql = SQL::raw("SELECT id, name FROM manual_users WHERE id = ")
//         .append(SQL::parameter(SQLiteValue::Integer(1)));

//     assert!(sql.sql().contains("SELECT"));
//     assert_eq!(sql.params().len(), 1);
// }

// #[test]
// fn manual_parameter_binding() {
//     let sql = SQL::raw("INSERT INTO test VALUES (")
//         .append(SQL::parameter(SQLiteValue::Integer(1)))
//         .append_raw(", ")
//         .append(SQL::parameter(SQLiteValue::Text("name".into())))
//         .append_raw(", ")
//         .append(SQL::parameter(SQLiteValue::Integer(25)))
//         .append_raw(")");

//     assert_eq!(sql.params().len(), 3);
//     assert!(sql.sql().contains("INSERT INTO test VALUES"));
// }

// #[test]
// fn manual_trait_implementation() {
//     let conn = Connection::open_in_memory().unwrap();

//     // Create table manually since we don't have the SQL constant from macros
//     conn.execute(
//         "CREATE TABLE manual_users (
//             id INTEGER PRIMARY KEY,
//             name TEXT NOT NULL,
//             email TEXT
//         )",
//         [],
//     )
//     .unwrap();

//     // Test manual ToSQL implementation
//     let insert_data = InsertUser {
//         name: "John".to_string(),
//         email: Some("john@test.com".to_string()),
//     };

//     let values_sql = insert_data.to_sql();
//     let insert_sql = SQL::raw("INSERT INTO manual_users (name, email) VALUES ").append(values_sql);

//     // Execute the manually built query
//     let sql_string = insert_sql.sql();
//     let params = insert_sql.params();

//     conn.execute(&sql_string, rusqlite::params_from_iter(params.iter()))
//         .unwrap();

//     // Verify it worked
//     let count: i32 = conn
//         .query_row("SELECT COUNT(*) FROM manual_users", [], |row| row.get(0))
//         .unwrap();

//     assert_eq!(count, 1);
// }

// #[test]
// fn manual_schema_trait() {
//     // Test that our manual schema implementation works
//     assert_eq!(ManualUserSchema::NAME, "manual_users");
//     assert_eq!(User::table_name(), "manual_users");
// }

// #[test]
// fn sql_fragment_composition() {
//     let select = SQL::raw("SELECT id, name");
//     let from = SQL::raw(" FROM users");
//     let where_clause =
//         SQL::raw(" WHERE active = ").append(SQL::parameter(SQLiteValue::Boolean(true)));
//     let order = SQL::raw(" ORDER BY name ASC");

//     let complete_query = select.append(from).append(where_clause).append(order);

//     assert_eq!(
//         complete_query.sql(),
//         "SELECT id, name FROM users WHERE active = ? ORDER BY name ASC"
//     );
//     assert_eq!(complete_query.params().len(), 1);
// }

// #[test]
// fn null_parameter_handling() {
//     let sql = SQL::raw("UPDATE users SET email = ")
//         .append(SQL::parameter(SQLiteValue::Null))
//         .append_raw(" WHERE id = ")
//         .append(SQL::parameter(SQLiteValue::Integer(1)));

//     assert_eq!(sql.params().len(), 2);

//     // Verify null parameter is properly stored
//     let params = sql.params();
//     match &params[0] {
//         SQLiteValue::Null => {} // Expected
//         _ => panic!("Expected null parameter"),
//     }
// }
