//! PostgreSQL enum tests

// Unit tests for macro-generated enum behavior (no database needed)
#[cfg(feature = "postgres")]
mod unit_tests {
    use crate::common::pg::*;
    use drizzle::prelude::*;

    #[test]
    fn test_enum_from_str() {
        let user: PgRole = "User".parse().expect("Should parse User");
        let admin: PgRole = "Admin".parse().expect("Should parse Admin");
        let moderator: PgRole = "Moderator".parse().expect("Should parse Moderator");

        assert!(matches!(user, PgRole::User));
        assert!(matches!(admin, PgRole::Admin));
        assert!(matches!(moderator, PgRole::Moderator));
    }

    #[test]
    fn test_enum_from_str_error() {
        let result: Result<PgRole, _> = "Invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_display() {
        assert_eq!(PgRole::User.to_string(), "User");
        assert_eq!(PgRole::Admin.to_string(), "Admin");
        assert_eq!(PgRole::Moderator.to_string(), "Moderator");
    }

    #[test]
    fn test_enum_default() {
        let default = PgRole::default();
        assert!(matches!(default, PgRole::User));
    }

    #[test]
    fn test_enum_from_i64() {
        let user = PgRole::try_from(0i64).expect("Should convert 0 to User");
        let admin = PgRole::try_from(1i64).expect("Should convert 1 to Admin");
        let moderator = PgRole::try_from(2i64).expect("Should convert 2 to Moderator");

        assert!(matches!(user, PgRole::User));
        assert!(matches!(admin, PgRole::Admin));
        assert!(matches!(moderator, PgRole::Moderator));
    }

    #[test]
    fn test_enum_to_i64() {
        let user: i64 = PgRole::User.into();
        let admin: i64 = PgRole::Admin.into();
        let moderator: i64 = PgRole::Moderator.into();

        assert_eq!(user, 0);
        assert_eq!(admin, 1);
        assert_eq!(moderator, 2);
    }
}

// Database execution tests for enum storage/retrieval
#[cfg(all(feature = "uuid", any(feature = "postgres-sync", feature = "tokio-postgres")))]
mod execution {
    use crate::common::pg::*;
    use drizzle::prelude::*;
    use drizzle_macros::postgres_test;

    #[derive(Debug, PostgresFromRow)]
    struct PgComplexResult {
        id: uuid::Uuid,
        name: String,
        active: bool,
    }

    #[derive(Debug, PostgresFromRow)]
    struct RoleResult {
        role: String,
    }

    postgres_test!(enum_insert_and_select, PgComplexSchema, {
        let PgComplexSchema { complex, .. } = schema;

        // Insert with different enum values
        let stmt = db.insert(complex).values([
            InsertPgComplex::new("Admin User", true, PgRole::Admin),
            InsertPgComplex::new("Regular User", true, PgRole::User),
            InsertPgComplex::new("Mod User", true, PgRole::Moderator),
        ]);
        drizzle_exec!(stmt.execute());

        // Select and verify enum was stored correctly
        let stmt = db.select(()).from(complex).order_by([OrderBy::asc(complex.name)]);
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "Admin User");
        assert_eq!(results[1].name, "Mod User");
        assert_eq!(results[2].name, "Regular User");
    });

    postgres_test!(enum_filter_by_value, PgComplexSchema, {
        let PgComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertPgComplex::new("Admin 1", true, PgRole::Admin),
            InsertPgComplex::new("Admin 2", true, PgRole::Admin),
            InsertPgComplex::new("User 1", true, PgRole::User),
        ]);
        drizzle_exec!(stmt.execute());

        // Filter by enum value
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, PgRole::Admin));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.starts_with("Admin")));
    });

    postgres_test!(enum_update, PgComplexSchema, {
        let PgComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([InsertPgComplex::new("Test User", true, PgRole::User)]);
        drizzle_exec!(stmt.execute());

        // Update enum value
        let stmt = db
            .update(complex)
            .set(UpdatePgComplex::default().with_role(PgRole::Admin))
            .r#where(eq(complex.name, "Test User"));
        drizzle_exec!(stmt.execute());

        // Verify update by filtering
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, PgRole::Admin));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Test User");
    });

    postgres_test!(enum_in_array_condition, PgComplexSchema, {
        let PgComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertPgComplex::new("Admin", true, PgRole::Admin),
            InsertPgComplex::new("Moderator", true, PgRole::Moderator),
            InsertPgComplex::new("User", true, PgRole::User),
        ]);
        drizzle_exec!(stmt.execute());

        // Filter by multiple enum values
        let stmt = db.select(()).from(complex).r#where(or([
            eq(complex.role, PgRole::Admin),
            eq(complex.role, PgRole::Moderator),
        ]));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Admin"));
        assert!(names.contains(&"Moderator"));
    });
}
