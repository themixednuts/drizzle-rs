use std::marker::PhantomData;

/// Shared builder wrapper used by all drivers.
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State, Db> {
    pub(crate) drizzle: &'a Db,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State)>,
}
