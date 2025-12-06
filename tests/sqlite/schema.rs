#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[SQLiteTable]
struct TestTable {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: Option<String>,
}

#[SQLiteTable(strict)]
struct StrictTable {
    #[integer(primary)]
    id: i32,
    #[text]
    content: String,
}

#[test]
fn table_sql() {
    let table = TestTable::new();
    let sql = table.sql().sql();
    assert_eq!(
        sql,
        "CREATE TABLE \"test_table\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, email TEXT);"
    );
}

#[test]
fn strict_table() {
    let table = StrictTable::new();
    let sql = table.sql().sql();
    assert_eq!(
        sql,
        "CREATE TABLE \"strict_table\" (id INTEGER PRIMARY KEY NOT NULL, content TEXT NOT NULL) STRICT;"
    );
}

#[test]
fn name_attribute() {
    let table = TestTable::new();
    let sql = table.sql().sql();
    assert_eq!(
        sql,
        "CREATE TABLE \"test_table\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, email TEXT);"
    );
}

#[test]
fn column_types() {
    let table = TestTable::new();
    let sql = table.sql().sql();
    assert_eq!(
        sql,
        "CREATE TABLE \"test_table\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, email TEXT);"
    );
}

// Schema derive tests
#[SQLiteTable(name = "users")]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    email: String,
    #[text]
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

sqlite_test!(test_schema_derive, AppTestSchema, {
    // Test table SQL generation
    let user = User::new();
    let user_sql = user.sql();
    assert_eq!(
        user_sql.sql(),
        "CREATE TABLE \"users\" (id INTEGER PRIMARY KEY NOT NULL, email TEXT NOT NULL, name TEXT NOT NULL);"
    );

    // Test index SQL generation
    let email_idx_sql = UserEmailIdx::default().sql();
    assert_eq!(
        email_idx_sql.sql(),
        "CREATE UNIQUE INDEX \"user_email_idx\" ON \"users\" (email)"
    );

    let name_idx_sql = UserNameIdx::default().sql();
    assert_eq!(
        name_idx_sql.sql(),
        "CREATE INDEX \"user_name_idx\" ON \"users\" (name)"
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
    let result = drizzle_exec!(db.insert(schema.user).values([insert_data]).execute());
    assert_eq!(result, 1);

    // Test that the indexes work (this would fail if indexes weren't created)
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(schema.user)
            .r#where(eq(schema.user.email, "test@example.com"))
            .all()
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
    let result = drizzle_exec!(db.insert(user).values([insert_data]).execute());
    assert_eq!(result, 1);

    // Query using the destructured table
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(user)
            .r#where(eq(user.email, "destructured@example.com"))
            .all()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "destructured@example.com");
    assert_eq!(users[0].name, "Destructured User");
});

// Multi-table schema with foreign key dependencies for deterministic ordering tests
#[SQLiteTable(name = "departments")]
struct Department {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
}

#[SQLiteTable(name = "employees")]
struct Employee {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[integer(references = Department::id)]
    department_id: i32,
    #[integer(references = Employee::id)]
    manager_id: Option<i32>, // Self-reference
}

#[SQLiteTable(name = "projects")]
struct Project {
    #[integer(primary)]
    id: i32,
    #[text]
    title: String,
    #[integer(references = Employee::id)]
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
    let statements = schema.create_statements();

    // Expected order: departments first (no deps), then employees (deps on departments),
    // then projects (deps on employees), with indexes after each table
    let expected = vec![
        // Department table first (no dependencies)
        "CREATE TABLE \"departments\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL);",
        // Employee table second (depends on Department)
        "CREATE TABLE \"employees\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, department_id INTEGER NOT NULL REFERENCES departments(id), manager_id INTEGER REFERENCES employees(id));",
        // Employee indexes after Employee table (alphabetical order)
        "CREATE INDEX \"employee_manager_idx\" ON \"employees\" (manager_id)",
        "CREATE INDEX \"employee_dept_idx\" ON \"employees\" (department_id)",
        // Project table last (depends on Employee)
        "CREATE TABLE \"projects\" (id INTEGER PRIMARY KEY NOT NULL, title TEXT NOT NULL, lead_id INTEGER NOT NULL REFERENCES employees(id));",
        // Project indexes after Project table
        "CREATE UNIQUE INDEX \"project_title_idx\" ON \"projects\" (title)",
    ];

    assert_eq!(
        statements.len(),
        expected.len(),
        "Number of statements should match"
    );

    for (i, (actual, expected)) in statements.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "Statement {} should match expected order",
            i
        );
    }

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
