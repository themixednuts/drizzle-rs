pub trait Strict: Default {
    const IS_STRICT: bool;
}

pub trait WithoutRowId: Default {
    const USE_ROWID: bool;
}
