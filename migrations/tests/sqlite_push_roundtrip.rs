//! Push round-trip test: a snapshot-shaped DDL (built exactly the way the
//! table/schema macros' runtime `to_snapshot` builds it) is rendered to
//! CREATE SQL, applied to a real SQLite database, introspected back through
//! the same raw queries the drivers use, and diffed against the original.
//!
//! The diff must be empty — any churn here means `drizzle push` would keep
//! recreating objects on every run.

use drizzle_migrations::sqlite::{
    SQLiteDDL,
    collection::diff_ddl,
    compute_migration,
    ddl::{
        Column, ForeignKey, Generated, GeneratedType, Index, IndexColumn, PrimaryKey, Table,
        UniqueConstraint,
    },
    introspect::{
        RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo, RawIntrospection, RawViewInfo,
        assemble_ddl, queries,
    },
};
use rusqlite::Connection;

/// Introspect through the exact raw queries the rusqlite driver uses.
fn introspect(conn: &Connection) -> SQLiteDDL {
    let tables: Vec<(String, Option<String>)> = conn
        .prepare(queries::TABLES_QUERY)
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let columns: Vec<RawColumnInfo> = conn
        .prepare(queries::COLUMNS_QUERY)
        .unwrap()
        .query_map([], |row| {
            Ok(RawColumnInfo {
                table: row.get(0)?,
                cid: row.get(1)?,
                name: row.get(2)?,
                column_type: row.get(3)?,
                not_null: row.get::<_, i32>(4)? != 0,
                default_value: row.get(5)?,
                pk: row.get(6)?,
                hidden: row.get(7)?,
                sql: row.get(8)?,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let indexes: Vec<RawIndexInfo> = conn
        .prepare(queries::INDEXES_QUERY)
        .unwrap()
        .query_map([], |row| {
            Ok(RawIndexInfo {
                table: row.get(0)?,
                name: row.get(1)?,
                unique: row.get::<_, i32>(2)? != 0,
                origin: row.get(3)?,
                partial: row.get::<_, i32>(4)? != 0,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let index_columns: Vec<RawIndexColumn> = conn
        .prepare(queries::INDEX_COLUMNS_QUERY)
        .unwrap()
        .query_map([], |row| {
            Ok(RawIndexColumn {
                index_name: row.get(0)?,
                seqno: row.get(1)?,
                cid: row.get(2)?,
                name: row.get(3)?,
                desc: row.get::<_, i32>(4)? != 0,
                coll: row.get(5)?,
                key: row.get::<_, i32>(6)? != 0,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let foreign_keys: Vec<RawForeignKey> = conn
        .prepare(queries::FOREIGN_KEYS_QUERY)
        .unwrap()
        .query_map([], |row| {
            Ok(RawForeignKey {
                table: row.get(0)?,
                id: row.get(1)?,
                seq: row.get(2)?,
                to_table: row.get(3)?,
                from_column: row.get(4)?,
                to_column: row.get(5)?,
                on_update: row.get(6)?,
                on_delete: row.get(7)?,
                r#match: row.get(8)?,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let views: Vec<RawViewInfo> = conn
        .prepare(queries::VIEWS_QUERY)
        .unwrap()
        .query_map([], |row| {
            Ok(RawViewInfo {
                name: row.get(0)?,
                sql: row.get(1)?,
            })
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let index_sql: Vec<(String, String)> = conn
        .prepare(queries::INDEX_SQL_QUERY)
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assemble_ddl(RawIntrospection {
        tables,
        columns,
        indexes,
        index_columns,
        foreign_keys,
        views,
        index_sql,
    })
}

/// Build a DDL shaped exactly like the schema macro's runtime `to_snapshot`:
/// - Table entities carry strict/without_rowid
/// - columns carry not_null/autoincrement/defaults/generated (macro stores
///   generated expressions pre-parenthesized), but no PK/unique flags
/// - ONE PrimaryKey entity per table (composite included)
/// - ForeignKey entities with macro-convention names
/// - UniqueConstraint entities for unique columns
fn snapshot_shaped_ddl() -> SQLiteDDL {
    let mut ddl = SQLiteDDL::new();

    // users: single-column INTEGER PK AUTOINCREMENT, unique email, defaults,
    // a virtual generated column.
    ddl.tables.push(Table::new("users"));
    ddl.columns.push(
        Column::new("users", "id", "integer")
            .not_null()
            .autoincrement(),
    );
    ddl.columns
        .push(Column::new("users", "name", "text").not_null());
    ddl.columns
        .push(Column::new("users", "email", "text").not_null());
    ddl.columns
        .push(Column::new("users", "score", "integer").default_value("0"));
    ddl.columns
        .push(Column::new("users", "note", "text").default_value("'hello'"));
    let mut name_len = Column::new("users", "name_len", "integer");
    name_len.generated = Some(Generated {
        // Macro canonical form: pre-parenthesized
        expression: "(length(name))".into(),
        gen_type: GeneratedType::Virtual,
    });
    ddl.columns.push(name_len);
    ddl.pks.push(PrimaryKey::from_strings(
        "users".to_string(),
        "users_pk".to_string(),
        vec!["id".to_string()],
    ));
    ddl.uniques.push(UniqueConstraint::from_strings(
        "users".to_string(),
        "users_email_unique".to_string(),
        vec!["email".to_string()],
    ));

    // posts: FK to users with CASCADE, plus an index.
    ddl.tables.push(Table::new("posts"));
    ddl.columns.push(
        Column::new("posts", "id", "integer")
            .not_null()
            .autoincrement(),
    );
    ddl.columns
        .push(Column::new("posts", "author_id", "integer").not_null());
    ddl.columns
        .push(Column::new("posts", "title", "text").not_null());
    ddl.pks.push(PrimaryKey::from_strings(
        "posts".to_string(),
        "posts_pk".to_string(),
        vec!["id".to_string()],
    ));
    ddl.fks.push(
        ForeignKey::from_strings(
            "posts".to_string(),
            "fk_posts_author_id_users_id_fk".to_string(),
            vec!["author_id".to_string()],
            "users".to_string(),
            vec!["id".to_string()],
        )
        .on_delete("CASCADE"),
    );
    ddl.indexes.push(Index::new(
        "posts",
        "posts_title_idx",
        vec![IndexColumn::new("title")],
    ));

    // pair: composite PK + FK + STRICT table option.
    ddl.tables.push(Table::new("pair").strict());
    ddl.columns
        .push(Column::new("pair", "a", "integer").not_null());
    ddl.columns
        .push(Column::new("pair", "b", "integer").not_null());
    ddl.columns
        .push(Column::new("pair", "user_id", "integer").not_null());
    ddl.pks.push(PrimaryKey::from_strings(
        "pair".to_string(),
        "pair_pk".to_string(),
        vec!["a".to_string(), "b".to_string()],
    ));
    ddl.fks.push(ForeignKey::from_strings(
        "pair".to_string(),
        "fk_pair_user_id_users_id_fk".to_string(),
        vec!["user_id".to_string()],
        "users".to_string(),
        vec!["id".to_string()],
    ));

    ddl
}

#[test]
fn push_roundtrip_self_diff_is_empty() {
    let desired = snapshot_shaped_ddl();

    // Render the CREATE statements and apply them to a real database.
    let migration = compute_migration(&SQLiteDDL::new(), &desired);
    let conn = Connection::open_in_memory().unwrap();
    for sql in &migration.sql_statements {
        conn.execute_batch(sql)
            .unwrap_or_else(|e| panic!("failed to execute:\n{sql}\nerror: {e}"));
    }

    // Introspect the live database and diff against the desired schema.
    let live = introspect(&conn);
    let diffs = diff_ddl(&live, &desired);
    assert!(
        diffs.is_empty(),
        "push round-trip must be a no-op, got diffs: {diffs:#?}"
    );

    // And the full migration path agrees: nothing to do.
    let followup = compute_migration(&live, &desired);
    assert!(
        followup.sql_statements.is_empty(),
        "follow-up push generated SQL: {:?}",
        followup.sql_statements
    );
}

#[test]
fn snapshot_shaped_ddl_self_diff_is_empty() {
    // Composite-PK + FK schema diffed against itself must be a no-op.
    let ddl = snapshot_shaped_ddl();
    let diffs = diff_ddl(&ddl, &ddl);
    assert!(diffs.is_empty(), "self-diff must be empty: {diffs:#?}");
}
