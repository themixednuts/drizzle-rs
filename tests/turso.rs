mod common;

#[cfg(feature = "turso")]
mod turso_tests {
    use drizzle_rs::{prelude::*, turso::Drizzle};
    use turso::{Builder, Connection};

    use crate::common::{
        Complex, ComplexSchema, InsertComplex, InsertSimple, Role, SelectComplex, SelectSimple,
        Simple, SimpleSchema, UpdateSimple, UserMetadata,
    };

    // Helper function to create a turso connection for testing
    // Note: This will need a real turso database URL in practice
    async fn setup_turso_connection() -> Connection {
        // For testing, you'll need to provide an actual turso database URL
        // This is a placeholder - replace with your actual turso database URL
        let url = std::env::var("TURSO_DATABASE_URL").unwrap_or_else(|_| ":memory:".to_string());

        let db = Builder::new_local(&url).build().await.unwrap();
        let conn = db.connect().unwrap();
        conn
    }

    async fn setup_test_tables(conn: &Connection) {
        // println!("Simple: {}", Simple::new().sql().sql());
        // println!("Complex: {}", Complex::new().sql().sql());
        // Create Simple table
        conn.execute(Simple::new().sql().sql().as_str(), ())
            .await
            .expect("Failed to create simple table");

        // Create Complex table
        conn.execute(Complex::new().sql().sql().as_str(), ())
            .await
            .expect("Failed to create complex table");
    }

    #[tokio::test]
    async fn test_basic_turso_insert_select() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Test basic insert
        let data = InsertSimple::new("turso_test");
        let inserted = db.insert(simple).values([data]).execute().await.unwrap();

        assert_eq!(inserted, 1);

        // Test basic select
        let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();

        assert!(selected.len() > 0);
        assert_eq!(selected[0].name, "turso_test");
    }

    #[tokio::test]
    async fn test_turso_get_single_row() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Insert test data
        let data = InsertSimple::new("single_row_test");
        db.insert(simple).values([data]).execute().await.unwrap();

        // Test get method
        let row: SelectSimple = db.select(()).from(simple).get().await.unwrap();

        assert_eq!(row.name, "single_row_test");
    }

    #[tokio::test]
    async fn test_turso_column_tuple_select() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Insert test data
        let data = InsertSimple::new("column_tuple_test");
        db.insert(simple).values([data]).execute().await.unwrap();

        // Test column tuple select (alternative to partial select for turso)
        let row: SelectSimple = db.select(()).from(simple).get().await.unwrap();

        assert_eq!(row.name, "column_tuple_test");
    }

    #[cfg(feature = "uuid")]
    #[tokio::test]
    async fn test_turso_complex_types() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, ComplexSchema { complex }) = Drizzle::new(conn, ComplexSchema::new());

        // Test complex type insertion
        let complex_data = InsertComplex::new("turso_complex", true, Role::User)
            .with_email("test@turso.com".to_string())
            .with_age(30);

        let inserted = db
            .insert(complex)
            .values([complex_data])
            .execute()
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        // Test complex type selection
        let selected: Vec<SelectComplex> = db.select(()).from(complex).all().await.unwrap();

        assert!(selected.len() > 0);
        assert_eq!(selected[0].name, "turso_complex");
        assert_eq!(selected[0].email, Some("test@turso.com".to_string()));
        assert_eq!(selected[0].age, Some(30));
        assert_eq!(selected[0].active, true);
        assert_eq!(selected[0].role, Role::User);
    }

    #[cfg(all(feature = "serde", feature = "uuid"))]
    #[tokio::test]
    async fn test_turso_json_fields() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, ComplexSchema { complex }) = Drizzle::new(conn, ComplexSchema::new());

        let metadata = UserMetadata {
            preferences: vec!["dark_mode".to_string(), "notifications".to_string()],
            last_login: Some("2025-08-12T10:00:00Z".to_string()),
            theme: "dark".to_string(),
        };

        let complex_data =
            InsertComplex::new("json_test", true, Role::Admin).with_metadata(metadata.clone());

        let inserted = db
            .insert(complex)
            .values([complex_data])
            .execute()
            .await
            .unwrap();
        assert_eq!(inserted, 1);

        // Test JSON field retrieval
        let selected: Vec<SelectComplex> = db.select(()).from(complex).all().await.unwrap();

        assert!(selected.len() > 0);
        assert_eq!(selected[0].name, "json_test");
        assert_eq!(selected[0].metadata, Some(metadata));
    }

    #[tokio::test]
    async fn test_turso_update_operations() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Insert initial data
        let data = InsertSimple::new("update_test");
        db.insert(simple).values([data]).execute().await.unwrap();

        // Test update
        let update_data = UpdateSimple::default().with_name("updated_test");
        let updated = db
            .update(simple)
            .set(update_data)
            .r#where(eq(simple.name, "update_test"))
            .execute()
            .await
            .unwrap();

        assert_eq!(updated, 1);

        // Verify update
        let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();
        assert_eq!(selected[0].name, "updated_test");
    }

    #[tokio::test]
    async fn test_turso_delete_operations() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Insert test data
        let data = InsertSimple::new("delete_test");

        let inserted: SelectSimple = db
            .insert(simple)
            .values([data])
            .returning(simple.columns())
            .get()
            .await
            .unwrap();

        println!("Inserted: {:?}", inserted);

        // Test delete
        let deleted = db
            .delete(simple)
            .r#where(eq(simple.name, "delete_test"))
            .execute()
            .await
            .unwrap();

        assert_eq!(deleted, 1);

        // Verify deletion
        let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();
        assert!(selected.is_empty());
    }

    #[tokio::test]
    async fn test_turso_error_handling() {
        let conn = setup_turso_connection().await;
        setup_test_tables(&conn).await;

        let (db, SimpleSchema { simple }) = Drizzle::new(conn, SimpleSchema::new());

        // Test error when trying to get from empty table
        let result: Result<SelectSimple, _> = db.select(()).from(simple).get().await;
        assert!(result.is_err());

        // Test error when trying to insert duplicate primary key
        let data1 = InsertSimple::new("test1").with_id(1);
        let data2 = InsertSimple::new("test2").with_id(1);

        db.insert(simple).values([data1]).execute().await.unwrap();
        let result = db.insert(simple).values([data2]).execute().await;
        assert!(result.is_err());
    }
}
