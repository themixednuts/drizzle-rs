use drizzle_core::error::{DrizzleError, Result};
use drizzle_migrations::{
    AppliedMigrationMetadata, MatchedMigrationMetadata, MigrateOutcome, Migration, Migrations,
    Tracking, match_applied_migration_metadata,
};
use drizzle_types::Dialect;
use mysql_common::{Row, prelude::FromValue};

const LOCK_TIMEOUT_SECONDS: u32 = 30;

pub(super) enum Effect {
    Initialize,
    Execute(String),
    Query(String),
}

pub(super) enum Step {
    Run(Effect),
    Done(Result<MigrateOutcome>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Preflight,
    Initialize,
    Acquire,
    Create,
    Columns,
    Legacy,
    AddName,
    AddAppliedAt,
    Backfill,
    Dirty,
    Applied,
    Start,
    Statement,
    Finish,
    Release,
    Done,
}

pub(super) struct Session {
    migrations: Migrations,
    phase: Phase,
    lock: Option<String>,
    locked: bool,
    terminal: Option<Result<MigrateOutcome>>,
    legacy: Vec<AppliedMigrationMetadata>,
    backfill: Vec<MatchedMigrationMetadata>,
    backfill_index: usize,
    add_name: bool,
    add_applied_at: bool,
    pending: Vec<Migration>,
    migration_index: usize,
    statement_index: usize,
    applied: Vec<String>,
}

impl Session {
    pub(super) fn new(migrations: &[Migration], tracking: Tracking) -> Self {
        Self {
            migrations: Migrations::with_tracking(migrations.to_vec(), Dialect::MySQL, tracking),
            phase: Phase::Preflight,
            lock: None,
            locked: false,
            terminal: None,
            legacy: Vec::new(),
            backfill: Vec::new(),
            backfill_index: 0,
            add_name: false,
            add_applied_at: false,
            pending: Vec::new(),
            migration_index: 0,
            statement_index: 0,
            applied: Vec::new(),
        }
    }

    pub(super) fn start(&self) -> Step {
        Step::Run(Effect::Query(
            "SELECT DATABASE(), @@SESSION.autocommit".into(),
        ))
    }

    pub(super) fn resume(&mut self, result: Result<Vec<Row>>) -> Step {
        if self.phase == Phase::Release {
            return self.released(result);
        }

        let next = match result {
            Ok(rows) => self.advance(rows),
            Err(error) if self.phase == Phase::Statement => {
                let migration = &self.pending[self.migration_index];
                Err(DrizzleError::Other(
                    format!(
                        "migration '{}' failed after its dirty marker was recorded: {error}",
                        migration.tag()
                    )
                    .into(),
                ))
            }
            Err(error) => Err(error),
        };

        match next {
            Ok(step) => step,
            Err(error) => self.fail(error),
        }
    }

    fn advance(&mut self, rows: Vec<Row>) -> Result<Step> {
        match self.phase {
            Phase::Preflight => self.preflight(rows),
            Phase::Initialize => self.acquire(),
            Phase::Acquire => self.acquired(rows),
            Phase::Create => self.columns(),
            Phase::Columns => self.legacy(rows),
            Phase::Legacy => self.upgrade(rows),
            Phase::AddName => self.added_name(),
            Phase::AddAppliedAt => self.backfill(),
            Phase::Backfill => self.backfilled(),
            Phase::Dirty => self.dirty(rows),
            Phase::Applied => self.pending(rows),
            Phase::Start | Phase::Statement => self.statement(),
            Phase::Finish => self.finished(),
            Phase::Release | Phase::Done => Err(DrizzleError::Other(
                "invalid MySQL migration session transition".into(),
            )),
        }
    }

    fn preflight(&mut self, rows: Vec<Row>) -> Result<Step> {
        let row = self.one(rows, "migration session preflight")?;
        let database = self
            .value::<Option<String>>(&row, 0, "selected database")?
            .ok_or_else(|| {
                DrizzleError::Other("MySQL connection has no selected database".into())
            })?;
        let autocommit = self.value::<u64>(&row, 1, "autocommit mode")?;
        if autocommit != 1 {
            return Err(DrizzleError::Other(
                "cannot run MySQL migrations with autocommit disabled".into(),
            ));
        }

        self.lock = Some(self.migrations.mysql_advisory_lock_name(&database));
        self.phase = Phase::Initialize;
        Ok(Step::Run(Effect::Initialize))
    }

    fn acquire(&mut self) -> Result<Step> {
        let name = self.lock.clone().ok_or_else(|| {
            DrizzleError::Other("MySQL migration lock name was not prepared".into())
        })?;
        let escaped = name.replace('\'', "''");
        self.phase = Phase::Acquire;
        Ok(Step::Run(Effect::Query(format!(
            "SELECT GET_LOCK('{escaped}', {LOCK_TIMEOUT_SECONDS})"
        ))))
    }

    fn acquired(&mut self, rows: Vec<Row>) -> Result<Step> {
        let row = self.one(rows, "GET_LOCK")?;
        let name = self.lock.clone().ok_or_else(|| {
            DrizzleError::Other("MySQL migration lock name was not prepared".into())
        })?;
        match self.value::<Option<i64>>(&row, 0, "GET_LOCK")? {
            Some(1) => {
                self.locked = true;
                self.phase = Phase::Create;
                Ok(Step::Run(Effect::Execute(
                    self.migrations.create_table_sql(),
                )))
            }
            Some(0) => {
                self.lock = None;
                Err(DrizzleError::Other(
                    format!("timed out after {LOCK_TIMEOUT_SECONDS} seconds waiting for MySQL migration lock '{name}'")
                    .into(),
                ))
            }
            Some(value) => {
                self.lock = None;
                Err(DrizzleError::Other(
                    format!("MySQL returned unexpected GET_LOCK result {value} for '{name}'")
                        .into(),
                ))
            }
            None => {
                self.lock = None;
                Err(DrizzleError::Other(
                    format!("MySQL did not acquire migration lock '{name}'").into(),
                ))
            }
        }
    }

    fn columns(&mut self) -> Result<Step> {
        let table = self.migrations.table_name().replace('\'', "''");
        self.phase = Phase::Columns;
        Ok(Step::Run(Effect::Query(format!(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = '{table}'"
        ))))
    }

    fn legacy(&mut self, rows: Vec<Row>) -> Result<Step> {
        let columns = self.strings(rows, "tracking column name")?;
        let has_name = columns.iter().any(|column| column == "name");
        let has_applied_at = columns.iter().any(|column| column == "applied_at");
        let filter = if has_name && has_applied_at {
            " WHERE `name` IS NULL"
        } else {
            ""
        };
        self.add_name = !has_name;
        self.add_applied_at = !has_applied_at;
        self.phase = Phase::Legacy;
        Ok(Step::Run(Effect::Query(format!(
            "SELECT CAST(id AS SIGNED), `hash`, `created_at` FROM {}{filter} ORDER BY id ASC",
            self.migrations.table_ident_sql()
        ))))
    }

    fn upgrade(&mut self, rows: Vec<Row>) -> Result<Step> {
        self.legacy = rows
            .iter()
            .map(|row| {
                Ok(AppliedMigrationMetadata {
                    id: self.value(row, 0, "migration id")?,
                    hash: self.value(row, 1, "migration hash")?,
                    created_at: self.value(row, 2, "migration created_at")?,
                })
            })
            .collect::<Result<_>>()?;

        if self.add_name {
            self.phase = Phase::AddName;
            return Ok(Step::Run(Effect::Execute(format!(
                "ALTER TABLE {} ADD COLUMN `name` TEXT NULL",
                self.migrations.table_ident_sql()
            ))));
        }

        if self.add_applied_at {
            self.phase = Phase::AddAppliedAt;
            return Ok(Step::Run(Effect::Execute(format!(
                "ALTER TABLE {} ADD COLUMN `applied_at` TIMESTAMP NULL DEFAULT NULL",
                self.migrations.table_ident_sql()
            ))));
        }

        self.backfill()
    }

    fn added_name(&mut self) -> Result<Step> {
        if self.add_applied_at {
            self.phase = Phase::AddAppliedAt;
            return Ok(Step::Run(Effect::Execute(format!(
                "ALTER TABLE {} ADD COLUMN `applied_at` TIMESTAMP NULL DEFAULT NULL",
                self.migrations.table_ident_sql()
            ))));
        }
        self.backfill()
    }

    fn backfill(&mut self) -> Result<Step> {
        if self.backfill.is_empty() && !self.legacy.is_empty() {
            self.backfill = match_applied_migration_metadata(self.migrations.all(), &self.legacy)
                .map_err(|error| DrizzleError::Other(error.to_string().into()))?;
        }
        if let Some(row) = self.backfill.get(self.backfill_index) {
            self.phase = Phase::Backfill;
            return Ok(Step::Run(Effect::Execute(
                self.migrations.backfill_migration_metadata_sql(row),
            )));
        }
        self.read_dirty()
    }

    fn backfilled(&mut self) -> Result<Step> {
        self.backfill_index += 1;
        self.backfill()
    }

    fn read_dirty(&mut self) -> Result<Step> {
        self.phase = Phase::Dirty;
        Ok(Step::Run(Effect::Query(self.migrations.dirty_names_sql())))
    }

    fn dirty(&mut self, rows: Vec<Row>) -> Result<Step> {
        let dirty = self.strings(rows, "migration name")?;
        if let Some(error) = self.migrations.interrupted_migration_error(&dirty) {
            return Err(DrizzleError::Other(error.to_string().into()));
        }
        self.phase = Phase::Applied;
        Ok(Step::Run(Effect::Query(
            self.migrations.applied_names_sql(),
        )))
    }

    fn pending(&mut self, rows: Vec<Row>) -> Result<Step> {
        let applied = self.strings(rows, "migration name")?;
        self.pending = self.migrations.pending(&applied).cloned().collect();
        if self.pending.is_empty() {
            return Ok(self.complete(MigrateOutcome::UpToDate));
        }
        self.start_migration()
    }

    fn start_migration(&mut self) -> Result<Step> {
        let migration = &self.pending[self.migration_index];
        self.statement_index = 0;
        self.phase = Phase::Start;
        Ok(Step::Run(Effect::Execute(
            self.migrations.record_migration_started_sql(migration),
        )))
    }

    fn statement(&mut self) -> Result<Step> {
        let migration = &self.pending[self.migration_index];
        while let Some(statement) = migration.statements().get(self.statement_index) {
            self.statement_index += 1;
            let statement = statement.trim();
            if !statement.is_empty() {
                self.phase = Phase::Statement;
                return Ok(Step::Run(Effect::Execute(statement.to_owned())));
            }
        }
        self.phase = Phase::Finish;
        Ok(Step::Run(Effect::Execute(
            self.migrations.record_migration_finished_sql(migration),
        )))
    }

    fn finished(&mut self) -> Result<Step> {
        let migration = &self.pending[self.migration_index];
        self.applied.push(migration.tag().to_owned());
        self.migration_index += 1;
        if self.migration_index < self.pending.len() {
            return self.start_migration();
        }
        let tags = core::mem::take(&mut self.applied);
        Ok(self.complete(MigrateOutcome::Applied { tags }))
    }

    fn complete(&mut self, outcome: MigrateOutcome) -> Step {
        self.terminal = Some(Ok(outcome));
        self.release()
    }

    fn fail(&mut self, error: DrizzleError) -> Step {
        if self.locked {
            self.terminal = Some(Err(error));
            self.release()
        } else {
            self.phase = Phase::Done;
            Step::Done(Err(error))
        }
    }

    fn release(&mut self) -> Step {
        let name = self
            .lock
            .as_ref()
            .expect("an acquired MySQL migration lock is present");
        debug_assert!(self.locked);
        let escaped = name.replace('\'', "''");
        self.phase = Phase::Release;
        Step::Run(Effect::Query(format!("SELECT RELEASE_LOCK('{escaped}')")))
    }

    fn released(&mut self, result: Result<Vec<Row>>) -> Step {
        let name = self
            .lock
            .take()
            .expect("an acquired MySQL migration lock is present");
        self.locked = false;
        let release = result.and_then(|rows| {
            let row = self.one(rows, "RELEASE_LOCK")?;
            match self.value::<Option<i64>>(&row, 0, "RELEASE_LOCK")? {
                Some(1) => Ok(()),
                Some(0) => Err(DrizzleError::Other(
                    format!("MySQL did not release migration lock '{name}'").into(),
                )),
                Some(value) => Err(DrizzleError::Other(
                    format!("MySQL returned unexpected RELEASE_LOCK result {value} for '{name}'")
                        .into(),
                )),
                None => Err(DrizzleError::Other(
                    format!(
                        "MySQL no longer recognizes migration lock '{name}' while releasing it"
                    )
                    .into(),
                )),
            }
        });
        self.phase = Phase::Done;
        let terminal = self
            .terminal
            .take()
            .expect("MySQL migration completion precedes lock release");
        Step::Done(match terminal {
            Err(error) => Err(error),
            Ok(outcome) => release.map(|()| outcome),
        })
    }

    fn one(&self, rows: Vec<Row>, operation: &str) -> Result<Row> {
        let mut rows = rows.into_iter();
        let row = rows.next().ok_or_else(|| {
            DrizzleError::Other(format!("MySQL {operation} returned no result row").into())
        })?;
        if rows.next().is_some() {
            return Err(DrizzleError::Other(
                format!("MySQL {operation} returned multiple result rows").into(),
            ));
        }
        Ok(row)
    }

    fn strings(&self, rows: Vec<Row>, field: &str) -> Result<Vec<String>> {
        rows.iter().map(|row| self.value(row, 0, field)).collect()
    }

    fn value<T: FromValue>(&self, row: &Row, index: usize, field: &str) -> Result<T> {
        match row.get_opt(index) {
            Some(Ok(value)) => Ok(value),
            Some(Err(error)) => Err(DrizzleError::Other(
                format!("MySQL returned an invalid value for {field}: {error}").into(),
            )),
            None => Err(DrizzleError::Other(
                format!("MySQL result is missing required column {field}").into(),
            )),
        }
    }
}
