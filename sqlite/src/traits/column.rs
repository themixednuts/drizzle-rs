use std::ops::Deref;

pub trait Autoincrement: Clone + Copy + Default + Sync + Send {
    const AUTOINCREMENT: bool;
}

pub trait SQLAutoIncrement: Clone + Sync + Send {
    type Value;

    fn autoincrement(self) -> Self::Value;
}

pub trait Column {
    fn name(&self) -> &'static str;
}

impl<C: Column> Column for Box<C> {
    fn name(&self) -> &'static str {
        self.deref().name()
    }
}

pub trait SQLiteMode: Default + Sync + Send + Copy + Clone + Eq + PartialEq {}
