use drizzle_core::error::Result;
use drizzle_migrations::{MigrateOutcome, Migration, Tracking};
use mysql::prelude::Queryable;

use super::{execute_request, initialize_session, query_request};
use crate::builder::mysql::migration::{Effect, Session, Step};

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
                Effect::Execute(sql) => {
                    execute_request(self.connection, &sql, &[]).map(|_| Vec::new())
                }
                Effect::Query(sql) => query_request(self.connection, &sql, &[]),
            };
            step = self.session.resume(result);
        }
    }
}
