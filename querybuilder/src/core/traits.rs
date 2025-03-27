use std::marker::PhantomData;

// Column traits for SQLite column attributes
pub trait PrimaryKey {
    const IS_PRIMARY_KEY: bool;
}

pub trait NotNull {
    const IS_NOT_NULL: bool;
}

pub trait Unique {
    const IS_UNIQUE: bool;
}

pub trait DefaultValue {
    type Value;
    const HAS_DEFAULT: bool;
    const DEFAULT_VALUE: Option<Self::Value>;
}

pub trait DefaultFn {
    type Value;
    const HAS_DEFAULT: bool;
    fn get_default(&self) -> Option<Self::Value>;
}

// Marker structs for trait implementations
pub struct IsPrimary;
pub struct NotPrimary;
pub struct NotNullable;
pub struct Nullable;
pub struct IsUnique;
pub struct NotUnique;
pub struct DefaultNotSet;
pub struct DefaultFnNotSet;

// Implementations for marker structs
impl PrimaryKey for IsPrimary {
    const IS_PRIMARY_KEY: bool = true;
}

impl PrimaryKey for NotPrimary {
    const IS_PRIMARY_KEY: bool = false;
}

impl NotNull for NotNullable {
    const IS_NOT_NULL: bool = true;
}

impl NotNull for Nullable {
    const IS_NOT_NULL: bool = false;
}

impl Unique for IsUnique {
    const IS_UNIQUE: bool = true;
}

impl Unique for NotUnique {
    const IS_UNIQUE: bool = false;
}

// Default value implementation for DefaultNotSet
impl DefaultValue for DefaultNotSet {
    type Value = ();
    const HAS_DEFAULT: bool = false;
    const DEFAULT_VALUE: Option<()> = None;
}

// DefaultFnNotSet - no default function
impl DefaultFn for DefaultFnNotSet {
    type Value = ();
    const HAS_DEFAULT: bool = false;
    fn get_default(&self) -> Option<Self::Value> {
        None
    }
}

pub struct DefaultFnSet<T, F: Fn() -> T = fn() -> T>(pub F, PhantomData<T>);

impl<T: Clone, F: Fn() -> T> DefaultFn for DefaultFnSet<T, F> {
    type Value = T;
    const HAS_DEFAULT: bool = true;
    fn get_default(&self) -> Option<Self::Value> {
        Some((self.0)())
    }
}

// Implementation for Default trait (for compatibility)
impl<T: Default + Clone> DefaultFnSet<T, fn() -> T> {
    pub fn default_value() -> Self {
        DefaultFnSet(|| T::default(), PhantomData)
    }
}

// Implementation for custom functions
impl<T, F: Fn() -> T> DefaultFnSet<T, F> {
    pub fn new(f: F) -> Self {
        DefaultFnSet(f, PhantomData)
    }
}
