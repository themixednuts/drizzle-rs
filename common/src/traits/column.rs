pub trait PrimaryKey: Clone + Copy + Default + Sync + Send {
    const IS_PRIMARY: bool;
}

pub trait NotNull: Clone + Copy + Default + Sync + Send {
    const IS_NOT_NULL: bool;
}

pub trait Unique: Clone + Copy + Default + Sync + Send {
    const IS_UNIQUE: bool;
}

pub trait DefaultValue: Clone + Copy + Default + Sync + Send {
    const HAS_DEFAULT: bool;
}

pub trait DefaultFn: Clone + Copy + Default + Sync + Send {
    const HAS_DEFAULT_FN: bool;
}

pub trait SQLPrimary: Clone + Sync + Send {
    type Value;
    fn primary(self) -> Self::Value;
}

pub trait SQLNotNull: Clone + Sync + Send {
    type Value;
    fn not_null(self) -> Self::Value;
}

pub trait SQLUnique: Clone + Sync + Send {
    type Value;

    fn unique(self, name: &'static str) -> Self::Value;
}

pub trait SQLDefault: Clone + Sync + Send {
    type DataType;
    type Value;

    fn default(self, value: Self::DataType) -> Self::Value;
}

pub trait SQLDefaultFn<D, F>: Clone + Sync + Send {
    type Value;
    fn default_fn(self, fun: F) -> Self::Value;
}

pub trait ColumnBuilder: Sync + Send {
    fn name(&self) -> &str;
    // fn build(self) -> Self;
}
