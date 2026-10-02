use drizzle_core::error::{QueryContext, Result, ResultExt};
use drizzle_migrations::{MigrateOutcome, Migration, Tracking};
use mysql::prelude::Queryable;

use super::{driver_error, initialize_session, query_request};
use crate::builder::mysql::migration::{Effect, Session, Step};

/// Runs migration SQL through the text protocol. The prepared-statement
/// protocol rejects stored-program DDL (`CREATE PROCEDURE`, `DROP TRIGGER`,
/// ...) with error 1295, and migration SQL never has parameters.
fn execute_text(connection: &mut impl Queryable, sql: &str) -> Result<()> {
    drizzle_core::drizzle_trace_query!(sql, 0);
    connection
        .query_drop(sql)
        .map_err(driver_error)
        .with_query(|| QueryContext::new::<()>(sql, &[]))
}

pub(super) struct Runner<'a, Connection> {
    connection: &'a mut Connection,
    session: Session,
}

impl<'a, Connection: Queryable> Runner<'a, Connection> {
    pub(super) fn new(
        connection: &'a mut Connection,
        migrations: &[Migration],
        tracking: Tracking,
    ) -> Self {
        Self {
            connection,
            session: Session::new(migrations, tracking),
        }
    }

    pub(super) fn run(mut self) -> Result<MigrateOutcome> {
        let mut step = self.session.start();
        loop {
            let effect = match step {
                Step::Run(effect) => effect,
                Step::Done(result) => return result,
            };
            let result = match effect {
                Effect::Initialize => initialize_session(self.connection).map(|()| Vec::new()),
                Effect::Execute(sql) => execute_text(self.connection, &sql).map(|()| Vec::new()),
                Effect::Query(sql) => query_request(self.connection, &sql, &[]),
            };
            step = self.session.resume(result);
        }
    }
}
