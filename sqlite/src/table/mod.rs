use std::marker::PhantomData;

use crate::traits::{
    column::Column,
    table::{Strict, WithoutRowId},
};

#[derive(Debug, Default)]
pub struct NotStrict {}

impl Strict for NotStrict {
    const IS_STRICT: bool = false;
}

#[derive(Debug, Default)]
pub struct IsStrict {}

impl Strict for IsStrict {
    const IS_STRICT: bool = true;
}

#[derive(Debug, Default)]
pub struct IsWithoutRowID {}

impl WithoutRowId for IsWithoutRowID {
    const USE_ROWID: bool = false;
}

#[derive(Debug, Default)]
pub struct IsWithRowID {}

impl WithoutRowId for IsWithRowID {
    const USE_ROWID: bool = true;
}

#[derive(Default)]
pub struct SQLiteTable<S: Strict = NotStrict, R: WithoutRowId = IsWithRowID> {
    name: &'static str,
    columns: Vec<Box<dyn Column>>,
    _strict: PhantomData<S>,
    _rowid: PhantomData<R>,
}

impl<S: Strict, R: WithoutRowId> SQLiteTable<S, R> {
    // pub fn build(name: &'static str) -> SQLiteTableBuilder {
    //     SQLiteTableBuilder {
    //         name,
    //         _strict: PhantomData,
    //         _rowid: PhantomData,
    //         ..Default::default()
    //     }
    // }

    pub fn columns(&self) {}
}

#[derive(Default)]
pub struct SQLiteTableBuilder<S: Strict = NotStrict, R: WithoutRowId = IsWithRowID> {
    name: &'static str,
    columns: Vec<Box<dyn Column>>,
    _strict: PhantomData<S>,
    _rowid: PhantomData<R>,
}

impl<S: Strict, R: WithoutRowId> SQLiteTableBuilder<S, R> {
    // pub fn new(name: &'static str) -> SQLiteTableBuilder<NotStrict, IsWithRowID> {
    //     SQLiteTableBuilder {
    //         name,
    //         _strict: PhantomData::<NotStrict>,
    //         _rowid: PhantomData::<IsWithRowID>,
    //         ..Default::default()
    //     }
    // }
    pub fn add_column<C>(mut self, column: C) -> Self
    where
        C: Column + 'static,
    {
        self.columns.push(Box::new(column));
        self
    }

    pub fn finalize(self) -> SQLiteTable<S, R> {
        SQLiteTable {
            name: self.name,
            columns: self.columns,
            _strict: self._strict,
            _rowid: self._rowid,
        }
    }
}
impl<R: WithoutRowId> SQLiteTableBuilder<NotStrict, R> {
    pub fn strict(self) -> SQLiteTableBuilder<IsStrict, R> {
        SQLiteTableBuilder {
            name: self.name,
            _strict: PhantomData,
            _rowid: PhantomData,
            ..Default::default()
        }
    }
}
impl<S: Strict> SQLiteTableBuilder<S, IsWithRowID> {
    pub fn without_rowid(self) -> SQLiteTableBuilder<S, IsWithoutRowID> {
        SQLiteTableBuilder {
            name: self.name,
            _strict: PhantomData,
            _rowid: PhantomData,
            ..Default::default()
        }
    }
}

pub fn sqlite_table(name: &'static str) -> SQLiteTableBuilder {
    SQLiteTableBuilder {
        name,
        _strict: PhantomData::<NotStrict>,
        _rowid: PhantomData::<IsWithRowID>,
        ..Default::default()
    }
}
