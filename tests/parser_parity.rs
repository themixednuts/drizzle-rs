//! Producer-parity: the schema **parser** and the derive **macros** must
//! produce the same snapshot for the same source.
//!
//! This file *is* its own fixture: the schema structs below are compiled by
//! the real macros (giving `Schema::to_snapshot()`), and the very same file
//! text is fed to `SchemaParser` via `include_str!`. The two producers'
//! DDL entity sets are then compared order-insensitively — so the parser can
//! never drift from what the macros actually emit without this test failing.
//!
//! Layout notes:
//! * The SQLite module comes first so the parser's `result.schema` (first
//!   schema struct in source order) is the SQLite one; the Postgres side is
//!   left unscoped, which matches runtime semantics because every Postgres
//!   item below is a schema member.
//! * The fixtures deliberately exercise the once-divergent shapes so parity
//!   locks them: FK targets with explicit `name = "..."` renames (both
//!   dialects), composite Postgres PKs (ONE entity per table), Postgres
//!   identity sequence options, index `method`/`where`/`concurrently`
//!   (implicit btree stays `None`), `UNIQUE ... NULLS NOT DISTINCT`, and a
//!   `#[postgres_enum(schema = "...")]` enum used across schemas. Opaque
//!   `query(...)` view definitions remain out of scope for the parser.

#[cfg(any(
    feature = "rusqlite",
    feature = "turso",
    feature = "libsql",
    feature = "postgres"
))]
mod parity_util {
    use std::collections::BTreeMap;

    /// Order-insensitive multiset comparison with a symmetric-difference
    /// report on mismatch.
    pub fn assert_entity_parity(label: &str, macro_side: Vec<String>, parser_side: Vec<String>) {
        let mut counts: BTreeMap<String, i64> = BTreeMap::new();
        for entity in &macro_side {
            *counts.entry(entity.clone()).or_default() += 1;
        }
        for entity in &parser_side {
            *counts.entry(entity.clone()).or_default() -= 1;
        }

        let only_macro: Vec<&String> = counts
            .iter()
            .filter(|(_, n)| **n > 0)
            .map(|(e, _)| e)
            .collect();
        let only_parser: Vec<&String> = counts
            .iter()
            .filter(|(_, n)| **n < 0)
            .map(|(e, _)| e)
            .collect();

        assert!(
            only_macro.is_empty() && only_parser.is_empty(),
            "{label}: parser and macro snapshots diverge\n\n\
             == entities only in the macro (Schema::to_snapshot) side ==\n{}\n\n\
             == entities only in the parser side ==\n{}\n",
            only_macro
                .iter()
                .map(|e| format!("  {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
            only_parser
                .iter()
                .map(|e| format!("  {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

// =============================================================================
// SQLite fixture + test
// =============================================================================

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
mod sqlite_parity {
    use drizzle::sqlite::prelude::*;

    #[SQLiteTable(strict)]
    pub struct Users {
        #[column(primary)]
        pub id: i64,
        #[column(unique)]
        pub email: String,
        #[column(default = "guest")]
        pub display_name: String,
        #[column(default_sql = "CURRENT_TIMESTAMP")]
        pub created_at: String,
        #[column(default_sql = "strftime('%s','now')")]
        pub updated_at: i64,
        #[column(collate = NOCASE)]
        pub nickname: Option<String>,
    }

    // Composite primary key.
    #[SQLiteTable]
    pub struct OrderItems {
        #[column(primary)]
        pub order_id: i64,
        #[column(primary)]
        pub item_no: i64,
        pub quantity: i64,
    }

    // Column FK with actions, table-level composite FK + UNIQUE + CHECK,
    // generated stored + virtual columns.
    #[SQLiteTable(
        FOREIGN_KEY(
            columns(order_id, item_no),
            references(OrderItems, order_id, item_no),
            on_delete = "CASCADE"
        ),
        UNIQUE(columns(first, last), name = "shipments_name_unique"),
        CHECK(expr = "weight >= 0")
    )]
    pub struct Shipments {
        #[column(primary)]
        pub id: i64,
        pub order_id: i64,
        pub item_no: i64,
        #[column(references = Users::id, on_delete = SET_NULL, on_update = CASCADE)]
        pub courier_id: Option<i64>,
        pub first: String,
        pub last: String,
        pub weight: i64,
        #[column(generated(stored, "first || ' ' || last"))]
        pub full_name: String,
        #[column(generated(virtual, "weight * 2"))]
        pub double_weight: i64,
    }

    // Renamed FK targets: the explicit table name on `Registry` and the
    // explicit column name on `code` must surface in `RegistryEntries`' FK
    // entities (constraint name, target table, and target columns).
    #[SQLiteTable(name = "registry_master")]
    pub struct Registry {
        #[column(primary)]
        pub id: i64,
        #[column(unique, name = "registry_code")]
        pub code: String,
    }

    #[SQLiteTable(FOREIGN_KEY(
        columns(reg_id, reg_code),
        references(Registry, id, code),
        on_update = "CASCADE"
    ))]
    pub struct RegistryEntries {
        #[column(primary)]
        pub id: i64,
        #[column(name = "entry_registry_id")]
        pub reg_id: i64,
        pub reg_code: String,
        #[column(references = Registry::code, on_delete = SET_NULL)]
        pub linked_code: Option<String>,
    }

    #[SQLiteIndex(unique, where = "email IS NOT NULL")]
    pub struct UsersEmailIdx(Users::email);

    #[SQLiteView(DEFINITION = "SELECT id, email FROM users")]
    pub struct UserEmails {
        pub id: i64,
        pub email: String,
    }

    #[derive(SQLiteSchema)]
    pub struct SqliteParitySchema {
        pub users: Users,
        pub order_items: OrderItems,
        pub shipments: Shipments,
        pub registry: Registry,
        pub registry_entries: RegistryEntries,
        pub users_email_idx: UsersEmailIdx,
        pub user_emails: UserEmails,
    }

    #[test]
    fn sqlite_producer_parity() {
        use drizzle::migrations::parser::SchemaParser;
        use drizzle::migrations::schema::{Schema as _, Snapshot};
        use drizzle_types::Dialect;

        let macro_snapshot = match SqliteParitySchema::new().to_snapshot() {
            Snapshot::Sqlite(s) => s,
            Snapshot::Postgres(_) => panic!("expected a SQLite snapshot"),
        };

        let parsed = SchemaParser::parse(include_str!("parser_parity.rs"));
        assert!(
            parsed.errors.is_empty(),
            "parser reported errors on the fixture: {:?}",
            parsed.errors
        );
        let parser_snapshot = match drizzle::migrations::Snapshot::from_parse_result(
            &parsed,
            Dialect::SQLite,
            None,
        ) {
            Snapshot::Sqlite(s) => s,
            Snapshot::Postgres(_) => panic!("expected a SQLite snapshot"),
        };

        super::parity_util::assert_entity_parity(
            "sqlite",
            macro_snapshot
                .ddl
                .iter()
                .map(|e| format!("{e:?}"))
                .collect(),
            parser_snapshot
                .ddl
                .iter()
                .map(|e| format!("{e:?}"))
                .collect(),
        );

        // Presence checks: parity alone can't tell "both sides emit X"
        // from "both sides dropped X". Lock the partial-index predicate and
        // renamed-FK-target shapes on the macro side (parity extends them to
        // the parser side).
        use drizzle::migrations::sqlite::SqliteEntity;
        let email_index = macro_snapshot.ddl.iter().find_map(|entity| match entity {
            SqliteEntity::Index(index) if index.name.as_ref() == "users_email_idx" => Some(index),
            _ => None,
        });
        assert_eq!(
            email_index.and_then(|index| index.where_clause.as_deref()),
            Some("email IS NOT NULL"),
            "compiled SQLite schema dropped the partial-index predicate"
        );

        let fks: Vec<_> = macro_snapshot
            .ddl
            .iter()
            .filter_map(|e| match e {
                SqliteEntity::ForeignKey(fk) if fk.table.as_ref() == "registry_entries" => Some(fk),
                _ => None,
            })
            .collect();
        assert!(
            fks.iter().any(|fk| {
                fk.name.as_ref()
                    == "fk_registry_entries_linked_code_registry_master_registry_code_fk"
                    && fk.table_to.as_ref() == "registry_master"
                    && fk
                        .columns_to
                        .iter()
                        .map(AsRef::as_ref)
                        .eq(["registry_code"])
            }),
            "renamed single-column FK target not resolved: {fks:?}"
        );
        assert!(
            fks.iter().any(|fk| {
                fk.name.as_ref()
                    == "fk_registry_entries_entry_registry_id_reg_code_registry_master_id_registry_code_fk"
                    && fk.columns.iter().map(AsRef::as_ref).eq(["entry_registry_id", "reg_code"])
                    && fk.columns_to.iter().map(AsRef::as_ref).eq(["id", "registry_code"])
            }),
            "renamed composite FK source/target not resolved: {fks:?}"
        );
    }
}

// =============================================================================
// Postgres fixture + test
// =============================================================================

#[cfg(feature = "postgres")]
mod postgres_parity {
    use drizzle::postgres::prelude::*;

    #[derive(PostgresEnum, Default, Clone, Copy, PartialEq, Debug)]
    pub enum AccountStatus {
        #[default]
        Active,
        Suspended,
        Closed,
    }

    // Enum created in a non-public schema, used by a table in another
    // schema (`Accounts` lives in `public`).
    #[derive(PostgresEnum, Default, Clone, Copy, PartialEq, Debug)]
    #[postgres_enum(schema = "auth")]
    pub enum AuthRole {
        #[default]
        Member,
        Admin,
    }

    // An enum that is its schema's ONLY occupant: both producers must emit
    // the Schema entity, or the generated CREATE TYPE cannot apply.
    #[derive(PostgresEnum, Default, Clone, Copy, PartialEq, Debug)]
    #[postgres_enum(schema = "lookup")]
    pub enum Currency {
        #[default]
        Usd,
        Eur,
    }

    // Multi-schema: this table lives in `auth`. The explicit column name on
    // `handle` must surface in FK entities that reference it.
    #[PostgresTable(schema = "auth")]
    pub struct AuthUsers {
        #[column(primary)]
        pub id: i64,
        #[column(unique)]
        pub email: String,
        #[column(unique, name = "auth_handle")]
        pub handle: String,
    }

    /// Account records.
    #[PostgresTable(RLS)]
    pub struct Accounts {
        #[column(primary, identity(by_default))]
        pub id: i64,
        /// Current lifecycle state.
        #[column(enum)]
        pub status: AccountStatus,
        #[column(enum)]
        pub role: AuthRole,
        pub tags: Vec<String>,
        #[column(default = "basic")]
        pub tier: String,
        #[column(references = AuthUsers::id, on_delete = CASCADE, deferrable)]
        pub owner_id: i64,
        #[column(references = AuthUsers::handle)]
        pub owner_handle: String,
    }

    // Composite primary key (ONE PrimaryKey entity), identity sequence
    // options, and a table-level UNIQUE with NULLS NOT DISTINCT.
    #[PostgresTable(UNIQUE(columns(label), nulls_not_distinct))]
    pub struct AccountEvents {
        #[column(primary)]
        pub account_id: i64,
        #[column(primary)]
        pub seq: i64,
        #[column(identity(
            always,
            start = 100,
            increment = 5,
            min_value = 10,
            max_value = 100000,
            cache = 10,
            cycle
        ))]
        pub audit_no: i64,
        pub label: Option<String>,
    }

    #[PostgresIndex(unique)]
    pub struct AuthUsersEmailIdx(AuthUsers::email);

    // Explicit method + partial predicate + CONCURRENTLY. Implicit-btree
    // indexes (like AuthUsersEmailIdx above) must keep `method: None`.
    #[PostgresIndex(method = "hash", concurrent, where = "tier <> 'basic'")]
    pub struct AccountsTierIdx(Accounts::tier);

    #[PostgresView(DEFINITION = "SELECT id, tier FROM accounts")]
    pub struct AccountTiers {
        pub id: i64,
        pub tier: String,
    }

    #[PostgresPolicy(
        NAME = "accounts_owner_only",
        AS = "PERMISSIVE",
        FOR = "SELECT",
        TO(authenticated),
        USING = "owner_id = current_setting('app.user_id')::bigint"
    )]
    pub struct AccountsPolicy(Accounts);

    #[derive(PostgresSchema)]
    pub struct PostgresParitySchema {
        pub account_status: AccountStatus,
        pub auth_role: AuthRole,
        pub currency: Currency,
        pub auth_users: AuthUsers,
        pub accounts: Accounts,
        pub account_events: AccountEvents,
        pub auth_users_email_idx: AuthUsersEmailIdx,
        pub accounts_tier_idx: AccountsTierIdx,
        pub account_tiers: AccountTiers,
        pub accounts_policy: AccountsPolicy,
    }

    #[test]
    fn postgres_producer_parity() {
        use drizzle::migrations::parser::SchemaParser;
        use drizzle::migrations::schema::{Schema as _, Snapshot};
        use drizzle_types::Dialect;

        let macro_snapshot = match PostgresParitySchema::new().to_snapshot() {
            Snapshot::Postgres(s) => s,
            Snapshot::Sqlite(_) => panic!("expected a Postgres snapshot"),
        };

        let parsed = SchemaParser::parse(include_str!("parser_parity.rs"));
        assert!(
            parsed.errors.is_empty(),
            "parser reported errors on the fixture: {:?}",
            parsed.errors
        );
        let parser_snapshot = match drizzle::migrations::Snapshot::from_parse_result(
            &parsed,
            Dialect::PostgreSQL,
            None,
        ) {
            Snapshot::Postgres(s) => s,
            Snapshot::Sqlite(_) => panic!("expected a Postgres snapshot"),
        };

        super::parity_util::assert_entity_parity(
            "postgres",
            macro_snapshot
                .ddl
                .iter()
                .map(|e| format!("{e:?}"))
                .collect(),
            parser_snapshot
                .ddl
                .iter()
                .map(|e| format!("{e:?}"))
                .collect(),
        );

        // Presence checks: parity alone can't tell "both sides emit X"
        // from "both sides dropped X". Lock the once-divergent shapes on the
        // macro side (parity extends them to the parser side).
        use drizzle::migrations::postgres::PostgresEntity;
        let ddl = &macro_snapshot.ddl;

        // ONE composite PrimaryKey entity for account_events.
        let event_pks: Vec<_> = ddl
            .iter()
            .filter_map(|e| match e {
                PostgresEntity::PrimaryKey(pk) if pk.table.as_ref() == "account_events" => Some(pk),
                _ => None,
            })
            .collect();
        assert_eq!(
            event_pks.len(),
            1,
            "expected exactly one PrimaryKey entity: {event_pks:?}"
        );
        assert!(
            event_pks[0].name.as_ref() == "account_events_pkey"
                && event_pks[0]
                    .columns
                    .iter()
                    .map(AsRef::as_ref)
                    .eq(["account_id", "seq"]),
            "composite PK shape wrong: {event_pks:?}"
        );

        // A schema whose only occupant is an enum still gets its Schema
        // entity (CREATE TYPE lookup.currency needs CREATE SCHEMA lookup).
        assert!(
            ddl.iter().any(|e| matches!(
                e,
                PostgresEntity::Schema(s) if s.name.as_ref() == "lookup"
            )),
            "enum-only schema `lookup` must be registered as a Schema entity"
        );

        // Identity sequence options survive into the runtime snapshot.
        let audit_no = ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Column(c)
                    if c.table.as_ref() == "account_events" && c.name.as_ref() == "audit_no" =>
                {
                    Some(c)
                }
                _ => None,
            })
            .expect("audit_no column");
        let identity = audit_no.identity.as_ref().expect("identity");
        assert_eq!(identity.start_with.as_deref(), Some("100"));
        assert_eq!(identity.increment.as_deref(), Some("5"));
        assert_eq!(identity.min_value.as_deref(), Some("10"));
        assert_eq!(identity.max_value.as_deref(), Some("100000"));
        assert_eq!(identity.cache, Some(10));
        assert_eq!(identity.cycle, Some(true));

        // Index method/where/concurrently; implicit btree stays None.
        let tier_idx = ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Index(i) if i.name.as_ref() == "accounts_tier_idx" => Some(i),
                _ => None,
            })
            .expect("accounts_tier_idx");
        assert_eq!(tier_idx.method.as_deref(), Some("hash"));
        assert_eq!(tier_idx.where_clause.as_deref(), Some("tier <> 'basic'"));
        assert!(tier_idx.concurrently);
        let email_idx = ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Index(i) if i.name.as_ref() == "auth_users_email_idx" => Some(i),
                _ => None,
            })
            .expect("auth_users_email_idx");
        assert_eq!(email_idx.method, None, "implicit btree must stay None");

        // Table-level UNIQUE keeps NULLS NOT DISTINCT.
        assert!(
            ddl.iter().any(|e| matches!(
                e,
                PostgresEntity::UniqueConstraint(u)
                    if u.table.as_ref() == "account_events" && u.nulls_not_distinct
            )),
            "nulls_not_distinct dropped"
        );

        // Enum schema + cross-schema enum column type_schema.
        assert!(
            ddl.iter().any(|e| matches!(
                e,
                PostgresEntity::Enum(en)
                    if en.name.as_ref() == "AuthRole" && en.schema.as_ref() == "auth"
            )),
            "enum schema dropped"
        );
        let role_col = ddl
            .iter()
            .find_map(|e| match e {
                PostgresEntity::Column(c)
                    if c.table.as_ref() == "accounts" && c.name.as_ref() == "role" =>
                {
                    Some(c)
                }
                _ => None,
            })
            .expect("accounts.role column");
        assert_eq!(role_col.type_schema.as_deref(), Some("auth"));

        // FK target column resolves the explicit rename on AuthUsers.handle.
        assert!(
            ddl.iter().any(|e| matches!(
                e,
                PostgresEntity::ForeignKey(fk)
                    if fk.name.as_ref() == "accounts_owner_handle_fkey"
                        && fk.columns_to.iter().map(AsRef::as_ref).eq(["auth_handle"])
            )),
            "renamed PG FK target not resolved"
        );
    }
}
