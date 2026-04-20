#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct TestTable {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    email: Option<String>,
}

#[SQLiteTable(STRICT)]
struct StrictTable {
    #[column(PRIMARY)]
    id: i32,
    content: String,
}

#[test]
fn table_sql() {
    assert_eq!(
        TestTable::create_table_sql(),
        "CREATE TABLE `test_table` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL,\n\t`email` TEXT\n);"
    );
}

#[test]
fn strict_table() {
    assert_eq!(
        StrictTable::create_table_sql(),
        "CREATE TABLE `strict_table` (\n\t`id` INTEGER PRIMARY KEY,\n\t`content` TEXT NOT NULL\n) STRICT;"
    );
}

#[test]
fn name_attribute() {
    assert_eq!(
        TestTable::create_table_sql(),
        "CREATE TABLE `test_table` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL,\n\t`email` TEXT\n);"
    );
}

#[test]
fn column_types() {
    assert_eq!(
        TestTable::create_table_sql(),
        "CREATE TABLE `test_table` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL,\n\t`email` TEXT\n);"
    );
}

// Schema derive tests
#[SQLiteTable(NAME = "users")]
struct User {
    #[column(PRIMARY)]
    id: i32,
    email: String,
    name: String,
}

#[SQLiteIndex(unique)]
struct UserEmailIdx(User::email);

#[SQLiteIndex]
struct UserNameIdx(User::name);

#[derive(SQLiteSchema)]
struct AppTestSchema {
    user: User,
    user_email_idx: UserEmailIdx,
    user_name_idx: UserNameIdx,
}

#[SQLiteView(
    NAME = "user_emails",
    DEFINITION = {
        let builder = drizzle::sqlite::builder::QueryBuilder::new::<AppTestSchema>();
        let AppTestSchema { user, .. } = AppTestSchema::new();
        builder.select((user.id, user.email)).from(user)
    }
)]
struct UserEmailsView {
    id: i32,
    email: String,
}

#[SQLiteView(DEFINITION = "SELECT id FROM users")]
struct DefaultNameView {
    id: i32,
}

#[SQLiteView(EXISTING, NAME = "existing_users")]
struct ExistingUsersView {
    id: i32,
    email: String,
}

#[derive(SQLiteFromRow, Debug, PartialEq, Eq)]
#[from(UserEmailsView)]
struct UserEmailRow {
    id: i32,
    email: String,
}

#[derive(SQLiteFromRow, Debug, PartialEq, Eq)]
struct UserEmailAliasRow {
    id: i32,
    email: String,
}

#[derive(SQLiteSchema)]
struct ViewTestSchema {
    user: User,
    user_emails: UserEmailsView,
    default_name_view: DefaultNameView,
    existing_users: ExistingUsersView,
}

#[drizzle::test]
fn test_schema_derive(db: &mut TestDb<AppTestSchema>, schema: AppTestSchema) {
    // Test table SQL generation (DDL-based format)
    let user_sql = User::create_table_sql();
    assert_eq!(
        user_sql,
        "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY,\n\t`email` TEXT NOT NULL,\n\t`name` TEXT NOT NULL\n);"
    );

    // Test index SQL generation (compile-time const SQL format)
    let email_idx_sql = UserEmailIdx::ddl_sql();
    assert_eq!(
        email_idx_sql,
        "CREATE UNIQUE INDEX \"user_email_idx\" ON \"users\" (\"email\")"
    );

    let name_idx_sql = UserNameIdx::ddl_sql();
    assert_eq!(
        name_idx_sql,
        "CREATE INDEX \"user_name_idx\" ON \"users\" (\"name\")"
    );

    // Test that we can get all schema items
    let (user_table, email_idx, name_idx) = schema.items();

    // Verify table
    assert_eq!(user_table.name(), "users");

    // Verify indexes
    assert_eq!(email_idx.name(), "user_email_idx");
    assert_eq!(name_idx.name(), "user_name_idx");
    assert!(email_idx.is_unique());
    assert!(!name_idx.is_unique());

    // Verify schema structure
    assert_eq!(schema.user.name(), "users");
    assert_eq!(schema.user_email_idx.name(), "user_email_idx");
    assert_eq!(schema.user_name_idx.name(), "user_name_idx");
    assert!(schema.user_email_idx.is_unique());
    assert!(!schema.user_name_idx.is_unique());
}

#[drizzle::test]
fn test_schema_with_drizzle_macro(db: &mut TestDb<AppTestSchema>, schema: AppTestSchema) {
    // Test that we can use the schema for queries
    let insert_data = InsertUser::new("test@example.com", "Test User");
    let result = drizzle_exec!(db.insert(schema.user).values([insert_data]) => execute);
    assert_eq!(result, 1);

    // Test that the indexes work (this would fail if indexes weren't created)
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(schema.user)
            .r#where(eq(schema.user.email, "test@example.com"))
            => all
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "test@example.com");
    assert_eq!(users[0].name, "Test User");
}

#[drizzle::test]
fn test_schema_destructuring(db: &mut TestDb<AppTestSchema>, schema: AppTestSchema) {
    // Test destructuring the schema into individual components
    let (user, _, _) = schema.into();

    // Test that we can use the destructured components
    let insert_data = InsertUser::new("destructured@example.com", "Destructured User");
    let result = drizzle_exec!(db.insert(user).values([insert_data]) => execute);
    assert_eq!(result, 1);

    // Query using the destructured table
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(user)
            .r#where(eq(user.email, "destructured@example.com"))
            => all
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "destructured@example.com");
    assert_eq!(users[0].name, "Destructured User");
}

#[drizzle::test]
fn test_schema_with_view(db: &mut TestDb<ViewTestSchema>, schema: ViewTestSchema) {
    let ViewTestSchema {
        user,
        user_emails,
        default_name_view,
        existing_users: _,
    } = schema;

    let insert_data = [
        InsertUser::new("a@example.com", "User A"),
        InsertUser::new("b@example.com", "User B"),
    ];
    let result = drizzle_exec!(db.insert(user).values(insert_data) => execute);
    assert_eq!(result, 2);

    let results: Vec<UserEmailRow> = drizzle_exec!(
        db.select(UserEmailRow::Select)
            .from(user_emails)
            .order_by(asc(user_emails.id))
            => all
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].email, "a@example.com");

    let view_sql = UserEmailsView::create_view_sql();
    assert_eq!(
        view_sql,
        r#"CREATE VIEW `user_emails` AS SELECT "users"."id", "users"."email" FROM "users";"#
    );

    assert_eq!(DefaultNameView::VIEW_NAME, "default_name_view");
    assert_eq!(default_name_view.name(), "default_name_view");

    let statements: Vec<_> = schema
        .create_statements()
        .expect("create statements")
        .collect();
    assert!(
        statements.iter().any(|sql| sql.contains("CREATE VIEW")),
        "Expected CREATE VIEW statement"
    );
    assert!(
        statements
            .iter()
            .any(|sql| sql.contains("default_name_view")),
        "Expected default name view statement"
    );
    assert!(
        !statements.iter().any(|sql| sql.contains("existing_users")),
        "Existing view should not be created"
    );
}

#[drizzle::test]
fn test_view_alias_in_from_clause(db: &mut TestDb<ViewTestSchema>, schema: ViewTestSchema) {
    let ViewTestSchema {
        user,
        user_emails,
        default_name_view: _,
        existing_users: _,
    } = schema;

    let insert_data = [
        InsertUser::new("a@example.com", "User A"),
        InsertUser::new("b@example.com", "User B"),
    ];
    let result = drizzle_exec!(db.insert(user).values(insert_data) => execute);
    assert_eq!(result, 2);

    struct UeTag;
    impl drizzle::core::Tag for UeTag {
        const NAME: &'static str = "ue";
    }

    let ue = UserEmailsView::alias::<UeTag>();
    let stmt = db
        .select((ue.id, ue.email))
        .from(ue)
        .r#where(eq(ue.email, "a@example.com"))
        .order_by([asc(ue.id)]);

    let sql = stmt.to_sql().sql();
    assert_eq!(
        sql,
        r#"SELECT "ue"."id", "ue"."email" FROM "user_emails" AS "ue" WHERE "ue"."email" = ? ORDER BY "ue"."id" ASC"#
    );

    let ue2 = UserEmailsView::alias::<UeTag>();
    let alias_stmt = db
        .select(UserEmailAliasRow::Select)
        .from(ue2)
        .r#where(eq(ue2.email, "a@example.com"))
        .order_by([asc(ue2.id)]);
    let results: Vec<UserEmailAliasRow> = drizzle_exec!(alias_stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].email, "a@example.com");

    // Keep schema value used in this test scope.
    let _ = user_emails;
}

// Multi-table schema with foreign key dependencies for deterministic ordering tests
#[SQLiteTable(NAME = "departments")]
struct Department {
    #[column(PRIMARY)]
    id: i32,
    name: String,
}

#[SQLiteTable(NAME = "employees")]
struct Employee {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    #[column(REFERENCES = Department::id)]
    department_id: i32,
    #[column(REFERENCES = Employee::id)]
    manager_id: Option<i32>, // Self-reference
}

#[SQLiteTable(NAME = "projects")]
struct Project {
    #[column(PRIMARY)]
    id: i32,
    title: String,
    #[column(REFERENCES = Employee::id)]
    lead_id: i32,
}

#[SQLiteIndex(unique)]
struct ProjectTitleIdx(Project::title);

#[SQLiteIndex]
struct EmployeeDeptIdx(Employee::department_id);

#[SQLiteIndex]
struct EmployeeManagerIdx(Employee::manager_id);

// Deliberately out-of-order schema: starts with index, then dependent tables first
#[derive(SQLiteSchema)]
struct ComplexTestSchema {
    // Start with an index (should be moved to after its table)
    project_title_idx: ProjectTitleIdx,
    // Put dependent table before its dependency
    project: Project,
    employee_manager_idx: EmployeeManagerIdx,
    employee: Employee,
    employee_dept_idx: EmployeeDeptIdx,
    // Put the base dependency table last
    department: Department,
}

#[drizzle::test]
fn test_deterministic_ordering(db: &mut TestDb<ComplexTestSchema>, schema: ComplexTestSchema) {
    // Get the create statements - this should be deterministically ordered
    let statements: Vec<_> = schema
        .create_statements()
        .expect("create statements")
        .collect();

    // Should have 6 statements: 3 tables + 3 indexes
    assert_eq!(
        statements.len(),
        6,
        "Should have 6 statements (3 tables + 3 indexes). Got: {:?}",
        statements
    );

    // Verify ordering: tables should come before their dependent tables
    // Find positions of each table
    let dept_pos = statements
        .iter()
        .position(|s| s.contains("departments") && s.contains("CREATE TABLE"));
    let emp_pos = statements
        .iter()
        .position(|s| s.contains("employees") && s.contains("CREATE TABLE"));
    let proj_pos = statements
        .iter()
        .position(|s| s.contains("projects") && s.contains("CREATE TABLE"));

    assert!(dept_pos.is_some(), "Department table should exist");
    assert!(emp_pos.is_some(), "Employee table should exist");
    assert!(proj_pos.is_some(), "Project table should exist");

    // Verify dependency order: departments < employees < projects
    assert!(
        dept_pos.unwrap() < emp_pos.unwrap(),
        "Department should come before Employee (dept has no deps, emp depends on dept)"
    );
    assert!(
        emp_pos.unwrap() < proj_pos.unwrap(),
        "Employee should come before Project (project depends on employee)"
    );

    // Verify indexes come after their tables
    let proj_idx_pos = statements
        .iter()
        .position(|s| s.contains("project_title_idx"));
    assert!(
        proj_idx_pos.is_some() && proj_idx_pos.unwrap() > proj_pos.unwrap(),
        "Project index should come after project table"
    );

    // Verify that foreign key relationships are properly detected
    let (
        _project_title_idx,
        project,
        _employee_manager_idx,
        employee,
        _employee_dept_idx,
        department,
    ) = schema.items();

    // Test Department (no dependencies) using TABLE_REF
    assert_eq!(department.name(), "departments");
    let dept_ref = &<Department as drizzle::core::DrizzleTable>::TABLE_REF;
    assert_eq!(dept_ref.dependency_names.len(), 0);

    // Test Employee (depends on Department and itself)
    assert_eq!(employee.name(), "employees");
    let emp_ref = &<Employee as drizzle::core::DrizzleTable>::TABLE_REF;
    assert_eq!(emp_ref.dependency_names.len(), 2); // Department and Employee (self-reference)
    // Dependencies should be sorted by name for deterministic order
    let mut emp_dep_names: Vec<&str> = emp_ref.dependency_names.to_vec();
    emp_dep_names.sort();
    assert_eq!(emp_dep_names[0], "departments");
    assert_eq!(emp_dep_names[1], "employees");

    // Test Project (depends on Employee)
    assert_eq!(project.name(), "projects");
    let proj_ref = &<Project as drizzle::core::DrizzleTable>::TABLE_REF;
    assert_eq!(proj_ref.dependency_names.len(), 1);
    assert_eq!(proj_ref.dependency_names[0], "employees");

    // Test foreign key column references via TABLE_REF
    let dept_id_col = &emp_ref.columns[2]; // department_id column
    assert_eq!(dept_id_col.name, "department_id");

    // FK info is now on TableRef.foreign_keys, not on ColumnRef
    let emp_fks = emp_ref.foreign_keys;
    let dept_fk = emp_fks
        .iter()
        .find(|fk| fk.source_columns.contains(&"department_id"))
        .unwrap();
    assert_eq!(dept_fk.target_table, "departments");
    assert_eq!(dept_fk.target_columns, &["id"]);

    let manager_id_col = &emp_ref.columns[3]; // manager_id column
    assert_eq!(manager_id_col.name, "manager_id");

    let mgr_fk = emp_fks
        .iter()
        .find(|fk| fk.source_columns.contains(&"manager_id"))
        .unwrap();
    assert_eq!(mgr_fk.target_table, "employees"); // Self-reference
    assert_eq!(mgr_fk.target_columns, &["id"]);

    let lead_id_col = &proj_ref.columns[2]; // lead_id column
    assert_eq!(lead_id_col.name, "lead_id");

    let proj_fks = proj_ref.foreign_keys;
    let lead_fk = proj_fks
        .iter()
        .find(|fk| fk.source_columns.contains(&"lead_id"))
        .unwrap();
    assert_eq!(lead_fk.target_table, "employees");
    assert_eq!(lead_fk.target_columns, &["id"]);
}

#[SQLiteTable(NAME = "cycle_a")]
struct CycleA {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = CycleB::id)]
    b_id: i32,
}

#[SQLiteTable(NAME = "cycle_b")]
struct CycleB {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = CycleC::id)]
    c_id: i32,
}

#[SQLiteTable(NAME = "cycle_c")]
struct CycleC {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = CycleA::id)]
    a_id: i32,
}

#[derive(SQLiteSchema)]
struct CycleSchema {
    a: CycleA,
    b: CycleB,
    c: CycleC,
}

#[test]
fn sqlite_cycle_reports_structured_error() {
    let schema = CycleSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected cycle detection error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Cyclic table dependency detected in SQLiteSchema"),
        "unexpected error: {err}"
    );
}

#[SQLiteTable(NAME = "dup_table")]
struct DuplicateTableOne {
    #[column(PRIMARY)]
    id: i32,
}

#[SQLiteTable(NAME = "dup_table")]
struct DuplicateTableTwo {
    #[column(PRIMARY)]
    id: i32,
}

#[derive(SQLiteSchema)]
struct DuplicateTableSchema {
    first: DuplicateTableOne,
    second: DuplicateTableTwo,
}

#[test]
fn sqlite_duplicate_table_reports_error() {
    let schema = DuplicateTableSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected duplicate table error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Duplicate table names detected in SQLiteSchema"),
        "unexpected error: {err}"
    );
}

#[SQLiteTable(NAME = "dup_idx_table")]
struct DuplicateIndexTable {
    #[column(PRIMARY)]
    id: i32,
    email: String,
}

#[SQLiteIndex]
struct DuplicateIndex(DuplicateIndexTable::email);

#[derive(SQLiteSchema)]
struct DuplicateIndexSchema {
    table: DuplicateIndexTable,
    idx1: DuplicateIndex,
    idx2: DuplicateIndex,
}

#[test]
fn sqlite_duplicate_index_reports_error() {
    let schema = DuplicateIndexSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected duplicate index error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Duplicate index 'duplicate_index' on table 'dup_idx_table' in SQLiteSchema"),
        "unexpected error: {err}"
    );
}

// =============================================================================
// View query DSL tests
// =============================================================================

#[cfg(feature = "uuid")]
mod view_query {
    use super::*;
    use crate::common::schema::sqlite::{
        Complex, InsertComplex, InsertPost, Post, Role, SelectComplex,
    };
    use uuid::Uuid;

    // 1. Simple view — basic column selection
    #[SQLiteView(
        query(select(Complex::id, Complex::name), from(Complex)),
        NAME = "vq_simple_view"
    )]
    struct VqSimpleView {
        id: Uuid,
        name: String,
    }

    // 2. Filtered view — WHERE clause
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name, Complex::email),
            from(Complex),
            filter(eq(Complex::active, true)),
        ),
        NAME = "vq_active_users"
    )]
    struct VqActiveUsersView {
        id: Uuid,
        name: String,
        email: Option<String>,
    }

    // 3. Join view — LEFT JOIN with condition
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name, Post::title),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
        ),
        NAME = "vq_user_posts"
    )]
    struct VqUserPostsView {
        id: Uuid,
        name: String,
        title: Option<String>,
    }

    // 4. Aggregate view with GROUP BY
    #[SQLiteView(
        query(
            select(Complex::name, count(Post::id)),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
            group_by(Complex::name),
        ),
        NAME = "vq_post_counts"
    )]
    struct VqPostCountsView {
        name: String,
        post_count: i32,
    }

    // 5. Order + limit + offset
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name),
            from(Complex),
            order_by(asc(Complex::name)),
            limit(10),
            offset(5),
        ),
        NAME = "vq_ordered_users"
    )]
    struct VqOrderedUsersView {
        id: Uuid,
        name: String,
    }

    // 6. Complex filter — AND/OR/IS_NULL
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name),
            from(Complex),
            filter(and(
                eq(Complex::active, true),
                or(gt(Complex::age, 0), is_null(Complex::age))
            )),
        ),
        NAME = "vq_complex_filter"
    )]
    struct VqComplexFilterView {
        id: Uuid,
        name: String,
    }

    // 7. Between expression — uses age (integer) instead of id (UUID)
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name, Complex::age),
            from(Complex),
            filter(between(Complex::age, 18, 65)),
        ),
        NAME = "vq_between_view"
    )]
    struct VqBetweenView {
        id: Uuid,
        name: String,
        age: Option<i32>,
    }

    // 8. Having clause
    #[SQLiteView(
        query(
            select(Complex::name, count(Post::id)),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
            group_by(Complex::name),
            having(gt(count(Post::id), 0)),
        ),
        NAME = "vq_having_view"
    )]
    struct VqHavingView {
        name: String,
        post_count: i32,
    }

    // 9. Inner join
    #[SQLiteView(
        query(
            select(Complex::name, Post::title),
            from(Complex),
            join(Post, eq(Complex::id, Post::author_id)),
        ),
        NAME = "vq_inner_join"
    )]
    struct VqInnerJoinView {
        name: String,
        title: String,
    }

    // 10. Desc ordering
    #[SQLiteView(
        query(
            select(Complex::id, Complex::name),
            from(Complex),
            filter(eq(Complex::active, true)),
            order_by(desc(Complex::name)),
        ),
        NAME = "vq_desc_view"
    )]
    struct VqDescView {
        id: Uuid,
        name: String,
    }

    #[derive(SQLiteSchema)]
    struct VqTestSchema {
        complex: Complex,
        post: Post,
        vq_simple_view: VqSimpleView,
        vq_active_users: VqActiveUsersView,
        vq_user_posts: VqUserPostsView,
        vq_post_counts: VqPostCountsView,
        vq_ordered_users: VqOrderedUsersView,
        vq_complex_filter: VqComplexFilterView,
        vq_between_view: VqBetweenView,
        vq_having_view: VqHavingView,
        vq_inner_join: VqInnerJoinView,
        vq_desc_view: VqDescView,
    }

    #[test]
    fn view_query_simple_const_sql() {
        assert_eq!(
            VqSimpleView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name" FROM "complex""#
        );
        assert_eq!(
            VqSimpleView::ddl_sql(),
            r#"CREATE VIEW "vq_simple_view" AS SELECT "complex"."id" AS "id", "complex"."name" AS "name" FROM "complex""#
        );
    }

    #[test]
    fn view_query_filter_const_sql() {
        assert_eq!(
            VqActiveUsersView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name", "complex"."email" AS "email" FROM "complex" WHERE "complex"."active" = 1"#
        );
    }

    #[test]
    fn view_query_join_const_sql() {
        assert_eq!(
            VqUserPostsView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name", "posts"."title" AS "title" FROM "complex" LEFT JOIN "posts" ON "complex"."id" = "posts"."author_id""#
        );
    }

    #[test]
    fn view_query_aggregate_const_sql() {
        assert_eq!(
            VqPostCountsView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."name" AS "name", COUNT("posts"."id") AS "post_count" FROM "complex" LEFT JOIN "posts" ON "complex"."id" = "posts"."author_id" GROUP BY "complex"."name""#
        );
    }

    #[test]
    fn view_query_order_limit_offset_const_sql() {
        assert_eq!(
            VqOrderedUsersView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name" FROM "complex" ORDER BY "complex"."name" ASC LIMIT 10 OFFSET 5"#
        );
    }

    #[test]
    fn view_query_complex_filter_const_sql() {
        assert_eq!(
            VqComplexFilterView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name" FROM "complex" WHERE ("complex"."active" = 1 AND ("complex"."age" > 0 OR "complex"."age" IS NULL))"#
        );
    }

    #[test]
    fn view_query_between_const_sql() {
        assert_eq!(
            VqBetweenView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name", "complex"."age" AS "age" FROM "complex" WHERE "complex"."age" BETWEEN 18 AND 65"#
        );
    }

    #[test]
    fn view_query_having_const_sql() {
        assert_eq!(
            VqHavingView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."name" AS "name", COUNT("posts"."id") AS "post_count" FROM "complex" LEFT JOIN "posts" ON "complex"."id" = "posts"."author_id" GROUP BY "complex"."name" HAVING COUNT("posts"."id") > 0"#
        );
    }

    #[test]
    fn view_query_inner_join_const_sql() {
        assert_eq!(
            VqInnerJoinView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."name" AS "name", "posts"."title" AS "title" FROM "complex" JOIN "posts" ON "complex"."id" = "posts"."author_id""#
        );
    }

    #[test]
    fn view_query_desc_const_sql() {
        assert_eq!(
            VqDescView::VIEW_DEFINITION_SQL,
            r#"SELECT "complex"."id" AS "id", "complex"."name" AS "name" FROM "complex" WHERE "complex"."active" = 1 ORDER BY "complex"."name" DESC"#
        );
    }

    #[drizzle::test]
    fn test_view_query_simple(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            vq_simple_view,
            ..
        } = schema;

        let insert_data = [
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", false, Role::User).with_email("bob@example.com"),
        ];
        let result = drizzle_exec!(db.insert(complex).values(insert_data) => execute);
        assert_eq!(result, 2);

        let results: Vec<SelectVqSimpleView> = drizzle_exec!(
            db.select(())
                .from(vq_simple_view)
                .order_by([asc(vq_simple_view.name)])
                => all
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[1].name, "Bob");
    }

    #[drizzle::test]
    fn test_view_query_filter(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            vq_active_users,
            ..
        } = schema;

        let insert_data = [
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", false, Role::User).with_email("bob@example.com"),
            InsertComplex::new("Charlie", true, Role::User).with_email("charlie@example.com"),
        ];
        drizzle_exec!(db.insert(complex).values(insert_data) => execute);

        let results: Vec<SelectVqActiveUsersView> = drizzle_exec!(
            db.select(())
                .from(vq_active_users)
                .order_by([asc(vq_active_users.name)])
                => all
        );

        assert_eq!(results.len(), 2, "Should only see active users");
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[1].name, "Charlie");
    }

    #[drizzle::test]
    fn test_view_query_join(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            post,
            vq_user_posts,
            ..
        } = schema;

        drizzle_exec!(db.insert(complex).values([
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", true, Role::User).with_email("bob@example.com"),
        ]) => execute);

        // Get inserted user IDs for FK references
        let users: Vec<SelectComplex> = drizzle_exec!(
            db.select(()).from(complex).order_by([asc(complex.name)]) => all
        );
        let alice_id = users[0].id;

        drizzle_exec!(db.insert(post).values([
            InsertPost::new("Post 1", true).with_author_id(alice_id),
            InsertPost::new("Post 2", false).with_author_id(alice_id),
        ]) => execute);

        let results: Vec<SelectVqUserPostsView> = drizzle_exec!(
            db.select(())
                .from(vq_user_posts)
                .order_by([asc(vq_user_posts.name), asc(vq_user_posts.title)])
                => all
        );

        // Alice has 2 posts, Bob has 0 (LEFT JOIN -> Bob row with NULL title)
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[0].title, Some("Post 1".to_string()));
        assert_eq!(results[1].name, "Alice");
        assert_eq!(results[1].title, Some("Post 2".to_string()));
        assert_eq!(results[2].name, "Bob");
        assert_eq!(results[2].title, None);
    }

    #[drizzle::test]
    fn test_view_query_aggregate(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            post,
            vq_post_counts,
            ..
        } = schema;

        drizzle_exec!(db.insert(complex).values([
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", true, Role::User).with_email("bob@example.com"),
        ]) => execute);

        // Get inserted user IDs for FK references
        let users: Vec<SelectComplex> = drizzle_exec!(
            db.select(()).from(complex).order_by([asc(complex.name)]) => all
        );
        let alice_id = users[0].id;
        let bob_id = users[1].id;

        drizzle_exec!(db.insert(post).values([
            InsertPost::new("Post A", true).with_author_id(alice_id),
            InsertPost::new("Post B", false).with_author_id(alice_id),
            InsertPost::new("Post C", true).with_author_id(bob_id),
        ]) => execute);

        let results: Vec<SelectVqPostCountsView> = drizzle_exec!(
            db.select(())
                .from(vq_post_counts)
                .order_by([desc(vq_post_counts.post_count)])
                => all
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[0].post_count, 2);
        assert_eq!(results[1].name, "Bob");
        assert_eq!(results[1].post_count, 1);
    }

    #[drizzle::test]
    fn test_view_query_complex_filter(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            vq_complex_filter,
            ..
        } = schema;

        drizzle_exec!(db.insert(complex).values([
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", false, Role::User).with_email("bob@example.com"),
            InsertComplex::new("Charlie", true, Role::User).with_email("charlie@example.com"),
        ]) => execute);

        let results: Vec<SelectVqComplexFilterView> = drizzle_exec!(
            db.select(())
                .from(vq_complex_filter)
                .order_by([asc(vq_complex_filter.name)])
                => all
        );

        // active=true AND (age>0 OR age IS NULL) — Alice and Charlie match
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[1].name, "Charlie");
    }

    #[drizzle::test]
    fn test_view_query_having(db: &mut TestDb<VqTestSchema>, schema: VqTestSchema) {
        let VqTestSchema {
            complex,
            post,
            vq_having_view,
            ..
        } = schema;

        drizzle_exec!(db.insert(complex).values([
            InsertComplex::new("Alice", true, Role::User).with_email("alice@example.com"),
            InsertComplex::new("Bob", true, Role::User).with_email("bob@example.com"),
            InsertComplex::new("Charlie", true, Role::User).with_email("charlie@example.com"),
        ]) => execute);

        // Get inserted user IDs for FK references
        let users: Vec<SelectComplex> = drizzle_exec!(
            db.select(()).from(complex).order_by([asc(complex.name)]) => all
        );
        let alice_id = users[0].id;
        let bob_id = users[1].id;

        // Only Alice and Bob get posts
        drizzle_exec!(db.insert(post).values([
            InsertPost::new("Post 1", true).with_author_id(alice_id),
            InsertPost::new("Post 2", true).with_author_id(bob_id),
        ]) => execute);

        let results: Vec<SelectVqHavingView> = drizzle_exec!(
            db.select(())
                .from(vq_having_view)
                .order_by([asc(vq_having_view.name)])
                => all
        );

        // HAVING COUNT(posts.id) > 0 — Charlie has 0 posts, so excluded
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Alice");
        assert_eq!(results[1].name, "Bob");
    }
}
