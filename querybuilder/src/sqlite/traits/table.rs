pub trait Strict: Default {
    const IS_STRICT: bool;
}

pub trait WithoutRowId: Default {
    const USE_ROWID: bool;
}

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
