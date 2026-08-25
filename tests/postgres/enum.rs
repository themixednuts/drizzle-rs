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

#[cfg(feature = "aws-data-api")]
mod aws_data_api_traits {
    use crate::common::schema::postgres::Role;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, drizzle::postgres::PostgresEnum)]
    #[repr(i64)]
    enum NumericRole {
        #[default]
        User = 0,
        Admin = 1,
    }

    fn assert_enum_row_traits<Row: ?Sized>()
    where
        Role: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
        Option<Role>: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
        NumericRole: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
        Option<NumericRole>: drizzle::core::FromDrizzleRow<Row> + drizzle::core::RowColumnList<Row>,
    {
    }

    #[test]
    fn postgres_enum_supports_aws_scalar_tuple_decoding() {
        assert_enum_row_traits::<drizzle::postgres::aws_data_api::Row>();
    }
}

// Database execution tests for enum storage/retrieval
#[cfg(all(
    feature = "uuid",
    any(feature = "postgres-sync", feature = "tokio-postgres")
))]
mod execution {
    use crate::common::schema::postgres::*;
    use drizzle::core::expr::*;
    use drizzle::postgres::prelude::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PostgresEnum)]
    #[repr(i64)]
    enum NumericRole {
        #[default]
        User = 0,
        Admin = 1,
        Moderator = 2,
    }

    #[PostgresTable(name = "numeric_enum_records")]
    struct NumericEnumRecord {
        #[column(primary, serial)]
        id: i32,
        #[column(enum)]
        role: NumericRole,
        #[column(enum)]
        optional_role: Option<NumericRole>,
    }

    #[derive(PostgresSchema)]
    struct NumericEnumSchema {
        records: NumericEnumRecord,
    }

    #[PostgresTable(name = "numeric_enum_parents")]
    struct NumericEnumParent {
        // Keep the repr enum first: optional relation decoding probes the
        // relation's first projected column for NULL.
        #[column(enum)]
        role: NumericRole,
        #[column(primary, serial)]
        id: i32,
    }

    #[PostgresTable(name = "numeric_enum_children")]
    struct NumericEnumChild {
        #[column(primary, serial)]
        id: i32,
        #[column(references = NumericEnumParent::id)]
        parent_id: Option<i32>,
    }

    #[derive(PostgresSchema)]
    struct NumericEnumRelationSchema {
        parents: NumericEnumParent,
        children: NumericEnumChild,
    }

    #[allow(dead_code)]
    #[derive(Debug, PostgresFromRow)]
    struct PgComplexResult {
        id: uuid::Uuid,
        name: String,
        active: bool,
    }

    #[drizzle::test]
    fn enum_insert_and_select(db: &mut TestDb<ComplexSchema>) {
        let ComplexSchema { complex, .. } = schema;

        // Insert with different enum values
        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin User", true, Role::Admin),
            InsertComplex::new("Regular User", true, Role::User),
            InsertComplex::new("Mod User", true, Role::Moderator),
        ]);
        stmt.execute();

        // Select and verify enum was stored correctly
        let stmt = db.select(()).from(complex).order_by([asc(complex.name)]);
        let results: Vec<PgComplexResult> = stmt.all();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "Admin User");
        assert_eq!(results[1].name, "Mod User");
        assert_eq!(results[2].name, "Regular User");
    }

    #[drizzle::test]
    fn enum_filter_by_value(db: &mut TestDb<ComplexSchema>) {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin 1", true, Role::Admin),
            InsertComplex::new("Admin 2", true, Role::Admin),
            InsertComplex::new("User 1", true, Role::User),
        ]);
        stmt.execute();

        // Filter by enum value
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, Role::Admin));
        let results: Vec<PgComplexResult> = stmt.all();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.starts_with("Admin")));
    }

    #[drizzle::test]
    fn enum_update(db: &mut TestDb<ComplexSchema>) {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db
            .insert(complex)
            .values([InsertComplex::new("Test User", true, Role::User)]);
        stmt.execute();

        // Update enum value
        let stmt = db
            .update(complex)
            .set(UpdateComplex::default().with_role(Role::Admin))
            .r#where(eq(complex.name, "Test User"));
        stmt.execute();

        // Verify update by filtering
        let stmt = db
            .select(())
            .from(complex)
            .r#where(eq(complex.role, Role::Admin));
        let results: Vec<PgComplexResult> = stmt.all();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Test User");
    }

    #[drizzle::test]
    fn enum_in_array_condition(db: &mut TestDb<ComplexSchema>) {
        let ComplexSchema { complex, .. } = schema;

        let stmt = db.insert(complex).values([
            InsertComplex::new("Admin", true, Role::Admin),
            InsertComplex::new("Moderator", true, Role::Moderator),
            InsertComplex::new("User", true, Role::User),
        ]);
        stmt.execute();

        let results: Vec<(String, Role)> = db
            .select((complex.name, complex.role))
            .from(complex)
            .r#where(in_array(complex.role, [Role::Admin, Role::Moderator]))
            .order_by([asc(complex.name)])
            .all();

        assert_eq!(
            results,
            vec![
                ("Admin".to_string(), Role::Admin),
                ("Moderator".to_string(), Role::Moderator),
            ]
        );
    }

    #[drizzle::test]
    fn integer_enum_filters_and_round_trips(db: &mut TestDb<NumericEnumSchema>) {
        let NumericEnumSchema { records } = schema;

        db.insert(records)
            .values([InsertNumericEnumRecord::new(NumericRole::User)])
            .execute();
        db.insert(records)
            .values([
                InsertNumericEnumRecord::new(NumericRole::Admin)
                    .with_optional_role(NumericRole::Admin),
                InsertNumericEnumRecord::new(NumericRole::Moderator)
                    .with_optional_role(NumericRole::Moderator),
            ])
            .execute();

        let rows: Vec<SelectNumericEnumRecord> = db
            .select(())
            .from(records)
            .order_by([asc(records.id)])
            .all();
        assert_eq!(rows[0].optional_role, None);
        assert_eq!(rows[1].optional_role, Some(NumericRole::Admin));
        assert_eq!(rows[2].optional_role, Some(NumericRole::Moderator));

        let roles: Vec<NumericRole> = db
            .select(records.role)
            .from(records)
            .r#where(in_array(
                records.role,
                [NumericRole::Admin, NumericRole::Moderator],
            ))
            .order_by([asc(records.id)])
            .all();

        assert_eq!(roles, vec![NumericRole::Admin, NumericRole::Moderator]);
    }

    #[cfg(feature = "query")]
    #[drizzle::test]
    fn integer_enum_decodes_in_optional_relation(db: &mut TestDb<NumericEnumRelationSchema>) {
        let NumericEnumRelationSchema { parents, children } = schema;

        db.insert(parents)
            .values([InsertNumericEnumParent::new(NumericRole::Admin)])
            .execute();
        let parent: SelectNumericEnumParent = db.select(()).from(parents).get();

        db.insert(children)
            .values([InsertNumericEnumChild::new().with_parent_id(parent.id)])
            .execute();
        db.insert(children)
            .values([InsertNumericEnumChild::new()])
            .execute();

        let rows = db
            .query(children)
            .with(children.parent())
            .order_by(asc(children.id))
            .find_many();

        assert_eq!(
            rows[0].parent.as_ref().map(|row| row.role),
            Some(NumericRole::Admin)
        );
        assert!(rows[1].parent.is_none());
    }
}
