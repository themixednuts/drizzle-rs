# Drizzle ORM RC5: MySQL dialect differences relevant to drizzle-rs

## Source and scope

This note uses the local Drizzle ORM repository at `E:\Projects\drizzle-orm` as the primary source. The checked-out `beta` worktree is RC4 (`748058e8`, tag `v1.0.0-rc.4`), not RC5. RC5 is available as the fetched remote branch `origin/rc5` at commit `1fe465e63bd44be538163bde61b98dd309e6342f`; every source citation below refers to that exact Git object, read without switching or modifying the worktree.

Citation form:

> `E:\Projects\drizzle-orm @ 1fe465e6 :: <path>:<lines>`

The implementation conclusion is straightforward: keep drizzle-rs's public query vocabulary shared (`select`, `insert`, `update`, `delete`, joins, predicates, ordering, limits, transactions), put MySQL spelling and capability differences in the MySQL dialect, and put transport/result normalization in driver sessions. Do not create parallel convenience APIs such as `select_all`. Drizzle ORM itself follows that split: the dialect builds SQL, while `MySqlSession`/driver sessions prepare and execute it.

## Capability matrix

| Area | MySQL RC5 behavior | Difference from PostgreSQL / SQLite | drizzle-rs implication |
|---|---|---|---|
| Identifiers | Backticks; embedded backticks doubled | PostgreSQL and SQLite use double quotes | Dialect-owned identifier writer |
| Bind parameters | Positional `?` | PostgreSQL uses `$1`, `$2`; SQLite also uses `?` | Reuse shared parameter AST; render per dialect |
| Schema qualifier | A MySQL "schema" is modeled as a database and renders `database.table` | PostgreSQL schema is a namespace; SQLite has no matching schema model | Keep a shared qualified-name type, but MySQL migration scope is one database |
| Native types | Unsigned numerics, `tinyint`/`mediumint`, inline `enum`, binary/blob families, `year`, charset/collation, `AUTO_INCREMENT`, timestamp `ON UPDATE` | PG has named enum types, arrays/ranges/extensions; SQLite has affinity types | MySQL schema/type module is necessary; do not force all types through common SQL markers |
| Insert conflict | `INSERT IGNORE` and `ON DUPLICATE KEY UPDATE`; server chooses the conflicting PK/unique index | PG/SQLite use `ON CONFLICT` with targets and can do nothing | Keep base insert fluent API shared; expose a MySQL-specific conflict clause or a typed cross-dialect upsert abstraction that does not pretend targets are portable |
| DML returning | No SQL `RETURNING`; RC5 offers `$returningId()` by reconstructing generated keys from driver metadata | PG/SQLite support arbitrary `RETURNING` for insert/update/delete | Do not emit `RETURNING` for MySQL. Return driver insert metadata through the normal execution result; only add a generated-ID helper if its limitations are explicit |
| Update/delete | `ORDER BY` and `LIMIT`; no RC5 `RETURNING` or update `FROM` | SQLite also has order/limit and returning; PG has returning/update-from but not this MySQL form | Capability-gate methods on the same builder rather than adding alternate builder names |
| Joins | left/right/inner/cross plus lateral forms; no full join | PG has full and lateral; SQLite RC5 has full but no lateral methods | Join kind should be a dialect capability; `full_join` must not compile/render for MySQL |
| Indexes | `USING btree/hash`, DDL algorithm, lock, query index hints; no partial-index predicate | PG/SQLite expose `.where(...)` partial indexes | Do not carry the new SQLite/PG partial-index API into MySQL |
| Locks | `FOR UPDATE` / `FOR SHARE`, optionally `NOWAIT` or `SKIP LOCKED` | PG exposes more lock strengths and an `OF` target; SQLite has none | Shared lock entry point can be capability-typed, with dialect-specific strength/config enums |
| Transactions | Isolation level, read-only/read-write, consistent snapshot; nested transactions use savepoints and require InnoDB | SQLite transaction behavior/options differ; PG options differ | Common `transaction` method, MySQL config type, driver-level capability validation |
| Introspection | Queries `INFORMATION_SCHEMA` for one database, including charset/collation, generated expressions and `EXTRA` | Metadata catalogs differ entirely | Separate MySQL introspector/snapshot lowering; share migration orchestration only |

## 1. Schema and SQL types

### Database-qualified tables, not PostgreSQL-style namespaces

RC5 exposes `mysqlDatabase(name)` (with `mysqlSchema` as an alias), and its tables retain a schema/database qualifier. The select dialect renders the qualifier before table/view names. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/schema.ts:6-20,28-46`; `drizzle-orm/src/mysql-core/dialect.ts:456-470,487-519`)

There is an important migration boundary: Drizzle Kit's MySQL schema conversion skips schema-qualified tables, and runtime migration config omits `migrationsSchema`. Introspection takes one `schema` string and applies it as `TABLE_SCHEMA`/`SCHEMA_NAME`. In practice, RC5 treats migration/introspection scope as one selected database even though runtime queries can qualify another database. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-kit/src/dialects/mysql/drizzle.ts:70-87`; `drizzle-orm/src/mysql-core/async/session.ts:268-273`; `drizzle-kit/src/dialects/mysql/introspect.ts:39-53,69-96,234-242`)

For drizzle-rs, model a qualified table name once and let the dialect render it. Do not reuse PostgreSQL schema DDL behavior for MySQL, and do not promise cross-database migration generation in the initial driver.

### MySQL needs its own type surface

The RC5 MySQL column catalog includes `tinyint`, `smallint`, `mediumint`, `int`, multiple bigint modes, decimal/float/double/real, `binary`/`varbinary`, four blob sizes, `char`/`varchar`, four text sizes, inline `mysqlEnum`, JSON, date/datetime/time/timestamp, `year`, and `serial`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/all.ts:1-63`)

The MySQL-only behaviors worth modeling rather than erasing are:

- signed versus unsigned numeric types and multiple bigint host representations (`number`, bigint, or string); (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/bigint.ts:6-16,29-37,45-50,63-71,84-120,127-143`)
- `AUTO_INCREMENT` as a column property, with `serial` built on it; (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/common.ts:117-146`; `drizzle-orm/src/mysql-core/columns/serial.ts:6-21,29-43`)
- native inline `enum('a','b')`, not PostgreSQL's separately named enum type; (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/enum.ts:7-41,82-120`)
- generated columns with `virtual` or `stored` mode; (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/common.ts:31-64`)
- string charset and collation; (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/string.common.ts:5-36`)
- datetime/timestamp fractional-second precision and `ON UPDATE CURRENT_TIMESTAMP`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/columns/date.common.ts:5-40`; `drizzle-kit/src/dialects/mysql/convertor.ts:31-52`)

A good drizzle-rs split is common semantic traits (`SqlType`, nullable, default, generated, primary/unique/reference) plus MySQL-specific concrete types/configuration. Avoid claiming that a PostgreSQL enum macro or SQLite text enum is the same DDL feature.

### Constraints and indexes

MySQL primary keys are not modeled with a user-supplied constraint name in RC5, while unique constraints may be named and otherwise receive a generated name. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/primary-keys.ts:5-54`; `drizzle-orm/src/mysql-core/unique-constraint.ts:6-12,40-69`)

MySQL index configuration supports `USING btree|hash`, `ALGORITHM default|inplace|copy`, and lock modes. It intentionally has no predicate/`where` member. PostgreSQL and SQLite index builders do expose `.where(condition)`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/indexes.ts:6-30,34-83`; `drizzle-orm/src/pg-core/indexes.ts:271-274`; `drizzle-orm/src/sqlite-core/indexes.ts:47-50`)

Therefore the existing drizzle-rs partial-index feature belongs to SQLite and PostgreSQL, not MySQL. MySQL's index-specific extensions are different and should live behind MySQL index traits/builders.

## 2. SQL generation, quoting, casing, and parameters

RC5's MySQL dialect:

- quotes identifiers with backticks and doubles embedded backticks;
- emits `?` for every positional parameter;
- escapes string literals by doubling single quotes.

PostgreSQL uses double-quoted identifiers and numbered `$n` parameters; SQLite uses double-quoted identifiers and `?`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/dialect.ts:94-103`; `drizzle-orm/src/pg-core/dialect.ts:90-99`; `drizzle-orm/src/sqlite-core/dialect.ts:82-91`)

RC5 applies configured snake/camel casing when column builders are attached to a table; explicit SQL column names remain explicit. This is orthogonal to quoting. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/casing.ts:1-25`; `drizzle-orm/src/mysql-core/table.ts:85-103`)

For drizzle-rs, the SQL AST should carry identifiers and parameters as structured nodes. The MySQL dialect should own backtick escaping and `?` rendering. Casing belongs at schema-definition/name-resolution time, not in ad hoc query methods.

## 3. Inserts, upserts, and returning

### Preserve the shared insert shape

RC5 retains the normal fluent shape: `insert(table).values(...)` and insert-from-select. Its MySQL builder validates non-empty value lists and ensures insert-select fields exist in the target table. RC5 additionally allows an explicit insert column list, but that is an optimization/control surface, not a reason to fork the whole API. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/insert.ts:104-180`; `drizzle-orm/src/mysql-core/dialect.ts:701-800`)

drizzle-rs should keep the same public operation names already used by SQLite/PostgreSQL. The MySQL dialect should lower the common insert AST to backtick identifiers, `default` entries, multi-row values, or insert-select.

### MySQL conflict semantics are not PostgreSQL conflict semantics

RC5 provides:

- `.ignore()` -> `INSERT IGNORE`;
- `.onDuplicateKeyUpdate({ set: ... })` -> `ON DUPLICATE KEY UPDATE ...`;
- no conflict target, because MySQL chooses the matching primary/unique key;
- no native do-nothing form other than `INSERT IGNORE` or a documented no-op update.

(`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/insert.ts:112-145,343-375`; `drizzle-orm/src/mysql-core/dialect.ts:802-811`)

PostgreSQL and SQLite generate `ON CONFLICT` and can carry conflict-target semantics. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/pg-core/dialect.ts:762-776`; `drizzle-orm/src/sqlite-core/dialect.ts:767-779`)

Do not silently translate a target-bearing `on_conflict(columns...)` into MySQL, because the target is not expressible. Either:

1. provide a common typed upsert abstraction whose MySQL lowering explicitly rejects/omits targets, or
2. keep the base insert API shared and add a clearly MySQL-specific `on_duplicate_key_update` extension.

The second choice is honest and still preserves the important common API. It is not the kind of redundant API fork represented by `select_all`.

### MySQL has no general DML returning in RC5

RC5's MySQL SQL builder never appends `RETURNING` to insert, update, or delete. PostgreSQL and SQLite dialects do for all three. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/dialect.ts:124-142,177-198,701-813`; `drizzle-orm/src/pg-core/dialect.ts:120-138,175-212,664-776`; `drizzle-orm/src/sqlite-core/dialect.ts:112-135,170-202,668-779`)

Instead, RC5 exposes `$returningId()`. It selects primary-key fields, then a response mapper reconstructs IDs from `insertId..insertId+affectedRows`, falling back to runtime-generated defaults recorded while building the insert. This is not arbitrary row returning and does not execute a second select. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/insert.ts:377-385`; `drizzle-orm/src/mysql-core/async/insert.ts:65-75`; `drizzle-orm/src/utils.ts:329-359`)

For drizzle-rs, keep `returning(...)` unavailable for MySQL at compile time. The driver's normal insert result should expose last insert ID and affected-row count. A future generated-ID convenience can sit on that result, but should not masquerade as SQL `RETURNING` or promise arbitrary columns.

## 4. Updates and deletes

The common operations remain `update(table).set(...).where(...)` and `delete(table).where(...)`. MySQL's RC5 dialect additionally emits `ORDER BY` and `LIMIT` on both update and delete, and applies runtime `onUpdateFn` values to omitted update columns. It does not model PostgreSQL-style update `FROM`, joined update, or any returning selection. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/dialect.ts:124-198`; `drizzle-orm/src/mysql-core/query-builders/update.ts:216-247`; `drizzle-orm/src/mysql-core/query-builders/delete.ts:157-188`)

SQLite RC5 shares order/limit and supports returning; PostgreSQL supports returning/update-from but has a different update/delete grammar. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/sqlite-core/dialect.ts:112-202`; `drizzle-orm/src/pg-core/dialect.ts:120-212`)

This argues for one update/delete builder vocabulary with capability-bounded methods. The dialect-specific marker should determine whether `returning`, `from`, `order_by`, and `limit` are available. No `mysql_update` or `delete_all` API is needed.

## 5. Selects, joins, set operators, hints, and locks

### Shared select pipeline

RC5's MySQL select dialect accepts the same main pieces as the other dialects: CTEs, selected fields, source, joins, where, group by, having, order by, limit, offset, distinct, and set operations. The dialect is also responsible for validating that selected columns come from the source/join graph. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/dialect.ts:389-454,534-580`)

This is the strongest evidence against a separate MySQL query API. Reuse drizzle-rs's existing select AST/builder and implement MySQL rendering plus capability checks.

### Join differences

RC5 MySQL exposes left, right, inner, and cross joins, plus lateral versions of left/inner/cross. It does not expose full join. PostgreSQL exposes full and lateral joins; SQLite RC5 exposes full join but no lateral builder methods. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/select.ts:423-448,488-616`; `drizzle-orm/src/pg-core/query-builders/select.ts:408-597`; `drizzle-orm/src/sqlite-core/query-builders/select.ts:323-438`)

drizzle-rs should represent join kind once, but only implement/expose supported kinds for each dialect. Do not emulate full join in the initial MySQL driver.

### Index hints are MySQL-specific

MySQL attaches `USE INDEX`, `FORCE INDEX`, and `IGNORE INDEX` hints to the base table and individual joins. RC5 accepts index/unique builders or strings, resolves them to names, and renders the hints next to table references. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/select.ts:51-74`; `drizzle-orm/src/mysql-core/dialect.ts:487-510,551-561`)

These belong in a MySQL extension on the shared select/join builder. They should not pollute generic SQL or be represented as raw strings at the final rendering boundary if typed index metadata is available.

### Set operators are shared, with MySQL-specific global ordering

RC5 exposes `union`, `union all`, `intersect`, `intersect all`, `except`, and `except all` for MySQL just as it does for PostgreSQL. The MySQL renderer strips table qualification from columns in the global set-operation `ORDER BY`, because qualified source-table columns are invalid there. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/select.ts:675-798,1215-1380`; `drizzle-orm/src/mysql-core/dialect.ts:650-698`)

That column dequalification belongs in MySQL dialect rendering. The shared AST should retain structured order expressions; do not force callers to handwrite raw SQL.

### RC5 behavior to verify rather than copy

Two RC5 renderings should not be treated as a correctness oracle:

- RC5 emits `OFFSET` whenever an offset is present, independently of `LIMIT`. MySQL's grammar only permits offset as part of `LIMIT {[offset,] row_count | row_count OFFSET offset}`. drizzle-rs should either require a limit, synthesize MySQL's unbounded-limit form, or reject a standalone offset. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/dialect.ts:544-549,687-698`; [MySQL 8.4 `SELECT` grammar](https://dev.mysql.com/doc/refman/8.4/en/select.html))
- RC5's MySQL `concat` helper renders `a || b`. MySQL treats `||` as logical OR by default and only treats it as concatenation under `PIPES_AS_CONCAT`; portable MySQL generation should use `CONCAT(a, b)`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/expressions.ts:8-10`; [MySQL operator precedence and SQL-mode behavior](https://dev.mysql.com/doc/refman/8.4/en/operator-precedence.html))

RC5 also exposes `INTERSECT`/`EXCEPT` variants without a server-version capability check. The syntax is valid in current MySQL 8.4, including `ALL`, but drizzle-rs should state its minimum supported MySQL version and exercise every exposed set operator in integration tests rather than assuming all deployed MySQL-compatible servers behave identically. ([MySQL 8.4 `EXCEPT` grammar](https://dev.mysql.com/doc/refman/8.4/en/except.html))

### Locking is related but not identical

MySQL's RC5 lock strengths are `update` and `share`, with `noWait` or `skipLocked`; the renderer emits `FOR <strength>` and the modifier. PostgreSQL has a broader lock model including target tables, while SQLite has no corresponding select-lock API. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/query-builders/select.types.ts:156-162`; `drizzle-orm/src/mysql-core/query-builders/select.ts:1086-1099`; `drizzle-orm/src/mysql-core/dialect.ts:563-575`)

Use a common lock-clause concept only if its strength/config type is dialect-associated. A single overly broad enum would permit invalid combinations.

## 6. Prepared queries and placeholders

RC5 separates placeholder identity from wire syntax. Named `Placeholder` nodes are resolved at execution into the positional parameter array; a missing value is an error, and column encoders still transform supplied values. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/sql/sql.ts:857-895`; `drizzle-orm/src/mysql-core/async/session.ts:127-149`)

MySQL select/insert/update/delete objects all expose `prepare()` and execute through the same session abstraction. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/async/select.ts:104-130`; `drizzle-orm/src/mysql-core/async/insert.ts:65-89`; `drizzle-orm/src/mysql-core/async/update.ts:53-72`; `drizzle-orm/src/mysql-core/async/delete.ts:53-72`)

For drizzle-rs:

- preserve the existing placeholder/prepared-query API;
- compile named placeholders to an ordered bind plan;
- render each occurrence as `?` for MySQL;
- let the driver encode values according to the associated column SQL type;
- reject missing named values before sending the query.

The distinction is important: MySQL and SQLite share `?` syntax, but their value codecs and result sessions remain different.

## 7. Driver sessions and result normalization

`MySqlSession` is the boundary between dialect SQL and a concrete driver. It standardizes prepare modes (`arrays`, `objects`, `raw`), query metadata, execution, and MySQL transaction SQL. Its raw mutation result shape contains `insertId` and `affectedRows`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/session.ts:8-35,37-77`)

The mysql2 adapter demonstrates the split:

- the dialect builds SQL;
- the adapter selects array/object row mode;
- raw mutation results are normalized to `{ insertId, affectedRows }`;
- the driver owns streaming/iteration;
- the transaction implementation owns connection checkout, begin/commit/rollback, and savepoints.

(`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql2/session.ts:55-110,147-216`)

Driver-specific codecs are also session/adapter concerns. RC5's mysql2 codec layer normalizes bigint, boolean, date/datetime/timestamp, decimal, float, binary, and blob results, and the driver enables large-number support before constructing the shared MySQL dialect/session. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql2/codecs.ts:1-51`; `drizzle-orm/src/mysql2/driver.ts:28-60`)

Other RC5 MySQL adapters return different native shapes but normalize them to the same session contract: PlanetScale maps `rowsAffected` and string `insertId`; TiDB maps `lastInsertId`/`rowsAffected` and validates unsupported transaction options. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/planetscale-serverless/session.ts:41-69`; `drizzle-orm/src/tidb-serverless/session.ts:18-40,65-96`)

drizzle-rs should mirror this boundary:

1. `drizzle-mysql` owns SQL grammar, schema types, codecs contracts, and query capabilities.
2. Each concrete Rust MySQL client feature owns connection/pool integration, parameter transport, row decoding, native result normalization, streaming, and transaction mechanics.
3. The umbrella `drizzle` crate re-exports the same high-level builders used by SQLite/PostgreSQL.

Do not bake one driver's row/result types into the dialect crate.

## 8. Transactions

RC5's MySQL transaction config includes isolation level, `read only`/`read write`, and `with consistent snapshot`. It generates `SET TRANSACTION ...` separately from `START TRANSACTION ...`. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/session.ts:51-55,79-100`)

The mysql2 session checks out a connection when needed, begins the transaction, commits on success, rolls back on failure, and releases pooled connections. Nested transactions use savepoints. RC5 explicitly notes that nested transactions/savepoints require InnoDB. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql2/session.ts:147-216`; `drizzle-orm/src/mysql-core/async/session.ts:241-265`)

Keep `db.transaction(...)` shared in drizzle-rs. Use a MySQL-specific transaction config and make the concrete client session responsible for support validation. Nested transaction support should be documented as engine-dependent rather than assumed from the SQL dialect alone.

## 9. Migrations and introspection

### Runtime migration execution

RC5's MySQL migrator:

- uses a configurable table defaulting to `__drizzle_migrations` with no migration-schema option;
- creates it with MySQL DDL;
- loads applied migrations;
- executes pending statements through the shared database transaction callback;
- records hash, timestamp, and name.

(`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/mysql-core/async/session.ts:268-345`; `drizzle-orm/src/mysql2/migrator.ts:1-13`)

The migration reader splits files at `--> statement-breakpoint`, which is shared infrastructure rather than MySQL SQL parsing. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-orm/src/migrator.ts:57-87`)

For drizzle-rs, reuse migration discovery/journaling but add a MySQL metadata-table strategy and MySQL executor. Do not assume that putting a sequence of MySQL DDL statements inside the transaction callback makes all server-side DDL atomic; the RC5 source shows orchestration, while MySQL documents that most object-defining/modifying DDL implicitly commits. ([MySQL 8.4 statements that cause an implicit commit](https://dev.mysql.com/doc/refman/8.4/en/implicit-commit.html))

### MySQL snapshot/introspection needs its own lowering

RC5 reads one database from `INFORMATION_SCHEMA.TABLES`, `STATISTICS`, `COLUMNS`, `SCHEMATA`, `TABLE_CONSTRAINTS`, `KEY_COLUMN_USAGE`, `REFERENTIAL_CONSTRAINTS`, and `CHECK_CONSTRAINTS`. It captures column type/default/nullability, character set/collation, generated expression, `EXTRA`, composite primary keys, foreign-key actions, checks, and indexes. (`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-kit/src/dialects/mysql/introspect.ts:39-140,195-250,330-452,454-485`)

MySQL DDL conversion uses MySQL-specific forms such as:

- backtick names;
- `AUTO_INCREMENT`;
- `GENERATED ALWAYS AS (...) VIRTUAL|STORED`;
- per-column character set/collation;
- composite `PRIMARY KEY` whose database name is `PRIMARY`;
- unique indexes;
- `DROP INDEX name ON table`;
- view algorithm, SQL security, and check option.

(`E:\Projects\drizzle-orm @ 1fe465e6 :: drizzle-kit/src/dialects/mysql/convertor.ts:20-87,166-179,222-253`)

The shared part of drizzle-rs migrations should be the intent model and orchestration. SQL serialization and introspection must be dialect modules; trying to reuse PostgreSQL catalog queries or SQLite table-rebuild logic would be incorrect.

## 10. Recommended implementation boundary for drizzle-rs

### Keep shared

- `select(...).from(...)`, CTEs, predicates, projections, aliases, grouping, ordering, pagination;
- `insert(table).values(...)` and insert-from-select;
- `update(table).set(...).where(...)`;
- `delete(table).where(...)`;
- prepare/execute and named placeholders;
- transaction callback shape;
- result-row mapping framework;
- migration journal/orchestration concepts.

### Make dialect capabilities, not alternate APIs

- identifier quoting and parameter spelling;
- supported join kinds and lock strengths;
- update/delete order/limit versus returning/from;
- arbitrary DML returning support;
- conflict/upsert grammar;
- partial-index predicates versus MySQL index algorithm/lock/using;
- native SQL types, generated/default clauses, charset/collation;
- migration DDL and introspection.

### Make driver-session responsibilities

- client/pool lifecycle;
- bind transport and row mode;
- type decoding/normalization;
- last-insert-ID and affected-row result normalization;
- streaming;
- transaction option validation and savepoints.

### API guardrails

1. Do not add `select_all`, `mysql_select`, or parallel CRUD entry points. The existing dialect-generic builders are the right user experience.
2. Do not expose `returning` on MySQL merely for surface parity. General MySQL returning is not present in RC5's SQL dialect.
3. Do not pretend PostgreSQL/SQLite conflict targets map to `ON DUPLICATE KEY UPDATE`.
4. Do not add partial-index `where` to MySQL; add MySQL index `using`/algorithm/lock separately if and when drizzle-rs supports those DDL features.
5. Do not bind the MySQL dialect crate to a single Rust client. Normalize client differences behind session traits/features.
6. Keep user-facing operation names and result ergonomics aligned with the existing SQLite/PostgreSQL drivers; express only real SQL capability differences through traits and associated types.

## Bottom line

Drizzle ORM RC5 does not implement MySQL by inventing a second ORM interface. It reuses the query-builder model and isolates four categories of difference: schema/type metadata, dialect SQL rendering, capability-specific extensions, and driver sessions. That is the correct model for drizzle-rs as well.

The largest non-portable features to design deliberately are native MySQL types, `ON DUPLICATE KEY UPDATE`, lack of arbitrary `RETURNING`, index hints/options with no partial indexes, MySQL transaction options, and client result normalization. Everything else should feel like the SQLite/PostgreSQL API the crate already has.
