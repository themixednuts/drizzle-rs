//! PostgreSQL enum tests

// Unit tests for macro-generated enum behavior (no database needed)
#[cfg(feature = "postgres")]
mod unit_tests {
    use crate::common::schema::postgres::*;

    #[test]
    fn test_enum_from_str() {
        let user: Role = "User".parse().expect("Should parse User");
        let admin: Role = "Admin".parse().expect("Should parse Admin");
        let moderator: Role = "Moderator".parse().expect("Should parse Moderator");

        assert!(matches!(user, Role::User));
        assert!(matches!(admin, Role::Admin));
        assert!(matches!(moderator, Role::Moderator));
    }

    #[test]
    fn test_enum_from_str_error() {
        let result: Result<Role, _> = "Invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_display() {
        assert_eq!(Role::User.to_string(), "User");
        assert_eq!(Role::Admin.to_string(), "Admin");
        assert_eq!(Role::Moderator.to_string(), "Moderator");
    }

    #[test]
    fn test_enum_default() {
        let default = Role::default();
        assert!(matches!(default, Role::User));
    }

    #[test]
    fn test_enum_from_i64() {
        let user = Role::try_from(0i64).expect("Should convert 0 to User");
        let admin = Role::try_from(1i64).expect("Should convert 1 to Admin");
        let moderator = Role::try_from(2i64).expect("Should convert 2 to Moderator");

        assert!(matches!(user, Role::User));
        assert!(matches!(admin, Role::Admin));
        assert!(matches!(moderator, Role::Moderator));
    }

    #[test]
    fn test_enum_to_i64() {
        let user: i64 = Role::User.into();
        let admin: i64 = Role::Admin.into();
        let moderator: i64 = Role::Moderator.into();

        assert_eq!(user, 0);
        assert_eq!(admin, 1);
        assert_eq!(moderator, 2);
    }
}

// Database execution tests for enum storage/retrieval
#[cfg(all(
    feature = "uuid",
    any(feature = "postgres-sync", feature = "tokio-postgres")
))]
mod execution {
    use crate::common::schema::postgres::*;
    use drizzle::core::conditions::*;
    use drizzle::postgres::prelude::*;
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

    postgres_test!(enum_insert_and_select, ComplexSchema, {
        let ComplexSchema { complex, .. } = schema;

        // Insert with different enum values
        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin User", true, Role::Admin),
            InsertComplex::new("Regular User", true, Role::User),
            InsertComplex::new("Mod User", true, Role::Moderator),
        ]);
        drizzle_exec!(stmt.execute());

        // Select and verify enum was stored correctly
        let stmt = db
            .select(())
            .from(complex)
            .order_by([OrderBy::asc(complex.name)]);
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "Admin User");
        assert_eq!(results[1].name, "Mod User");
        assert_eq!(results[2].name, "Regular User");
    });

    postgres_test!(enum_filter_by_value, ComplexSchema, {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin 1", true, Role::Admin),
            InsertComplex::new("Admin 2", true, Role::Admin),
            InsertComplex::new("User 1", true, Role::User),
        ]);
        drizzle_exec!(stmt.execute());

        // Filter by enum value
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, Role::Admin));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.starts_with("Admin")));
    });

    postgres_test!(enum_update, ComplexSchema, {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db
            .insert(complex)
            .values([InsertComplex::new("Test User", true, Role::User)]);
        drizzle_exec!(stmt.execute());

        // Update enum value
        let stmt = db
            .update(complex)
            .set(UpdateComplex::default().with_role(Role::Admin))
            .r#where(eq(complex.name, "Test User"));
        drizzle_exec!(stmt.execute());

        // Verify update by filtering
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, Role::Admin));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Test User");
    });

    postgres_test!(enum_in_array_condition, ComplexSchema, {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin", true, Role::Admin),
            InsertComplex::new("Moderator", true, Role::Moderator),
            InsertComplex::new("User", true, Role::User),
        ]);
        drizzle_exec!(stmt.execute());

        // Filter by multiple enum values
        let stmt = db.select(()).from(complex).r#where(or([
            eq(complex.role, Role::Admin),
            eq(complex.role, Role::Moderator),
        ]));
        let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Admin"));
        assert!(names.contains(&"Moderator"));
    });
}
