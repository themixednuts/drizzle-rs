#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

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

#[derive(SQLiteSchema)]
struct ViewTestSchema {
    user: User,
    user_emails: UserEmailsView,
    default_name_view: DefaultNameView,
    existing_users: ExistingUsersView,
}

sqlite_test!(test_schema_derive, AppTestSchema, {
    // Test table SQL generation (DDL-based format)
    let user_sql = User::create_table_sql();
    assert_eq!(
        user_sql,
        "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY,\n\t`email` TEXT NOT NULL,\n\t`name` TEXT NOT NULL\n);"
    );

    // Test index SQL generation
    let email_idx_sql = UserEmailIdx.ddl().sql();
    assert_eq!(
        email_idx_sql,
        "CREATE UNIQUE INDEX `user_email_idx` ON `users`(`email`);"
    );

    let name_idx_sql = UserNameIdx.ddl().sql();
    assert_eq!(
        name_idx_sql,
        "CREATE INDEX `user_name_idx` ON `users`(`name`);"
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
});

sqlite_test!(test_schema_with_drizzle_macro, AppTestSchema, {
    // Test that we can use the schema for queries
    let insert_data = InsertUser::new("test@example.com", "Test User");
    let result = drizzle_exec!(db.insert(schema.user).values([insert_data]) => execute);
    assert_eq!(result, 1);

    // Test that the indexes work (this would fail if indexes weren't created)
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(schema.user)
            .r#where(eq(schema.user.email, "test@example.com"))
            => all_as
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "test@example.com");
    assert_eq!(users[0].name, "Test User");
});

sqlite_test!(test_schema_destructuring, AppTestSchema, {
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
            => all_as
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "destructured@example.com");
    assert_eq!(users[0].name, "Destructured User");
});

sqlite_test!(test_schema_with_view, ViewTestSchema, {
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

    let results: Vec<SelectUserEmailsView> = drizzle_exec!(
        db.select((user_emails.id, user_emails.email))
            .from(user_emails)
            .order_by(asc(user_emails.id))
            => all_as
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
});

sqlite_test!(test_view_alias_in_from_clause, ViewTestSchema, {
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

    let ue = UserEmailsView::alias("ue");
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

    let results: Vec<SelectUserEmailsView> = drizzle_exec!(stmt => all_as);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].email, "a@example.com");

    // Keep schema value used in this test scope.
    let _ = user_emails;
});

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

sqlite_test!(test_deterministic_ordering, ComplexTestSchema, {
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

    // Test Department (no dependencies)
    assert_eq!(department.name(), "departments");
    assert_eq!(department.dependencies().len(), 0);

    // Test Employee (depends on Department and itself)
    assert_eq!(employee.name(), "employees");
    let emp_deps = employee.dependencies();
    assert_eq!(emp_deps.len(), 2); // Department and Employee (self-reference)
                                   // Dependencies should be sorted by name for deterministic order
    assert_eq!(emp_deps[0].name(), "departments");
    assert_eq!(emp_deps[1].name(), "employees");

    // Test Project (depends on Employee)
    assert_eq!(project.name(), "projects");
    let proj_deps = project.dependencies();
    assert_eq!(proj_deps.len(), 1);
    assert_eq!(proj_deps[0].name(), "employees");

    // Test foreign key column references
    let dept_id_col = &employee.columns()[2]; // department_id column
    assert_eq!(dept_id_col.name(), "department_id");

    if let Some(fk_col) = dept_id_col.foreign_key() {
        assert_eq!(fk_col.name(), "id");
        assert_eq!(fk_col.table().name(), "departments");
    } else {
        panic!("department_id should have foreign key reference");
    }

    let manager_id_col = &employee.columns()[3]; // manager_id column
    assert_eq!(manager_id_col.name(), "manager_id");

    if let Some(fk_col) = manager_id_col.foreign_key() {
        assert_eq!(fk_col.name(), "id");
        assert_eq!(fk_col.table().name(), "employees"); // Self-reference
    } else {
        panic!("manager_id should have foreign key reference");
    }

    let lead_id_col = &project.columns()[2]; // lead_id column
    assert_eq!(lead_id_col.name(), "lead_id");

    if let Some(fk_col) = lead_id_col.foreign_key() {
        assert_eq!(fk_col.name(), "id");
        assert_eq!(fk_col.table().name(), "employees");
    } else {
        panic!("lead_id should have foreign key reference");
    }
});

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
