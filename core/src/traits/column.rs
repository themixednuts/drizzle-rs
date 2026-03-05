use crate::expr::Expr;
use crate::{SQLParam, SQLSchema, SQLSchemaType, SQLTable, SQLTableInfo};

pub trait SQLColumnInfo: Send + Sync {
    fn is_not_null(&self) -> bool;
    fn is_primary_key(&self) -> bool;
    fn is_unique(&self) -> bool;
    fn name(&self) -> &'static str;
    fn r#type(&self) -> &'static str;
    fn has_default(&self) -> bool;

    fn table(&self) -> &'static dyn SQLTableInfo;
}

/// Column trait tying expression lifetimes to parameter values via `'a`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL column for this dialect",
    label = "ensure this column's table was derived with #[SQLiteTable] or #[PostgresTable]"
)]
pub trait SQLColumn<'a, Value: SQLParam + 'a>:
    SQLColumnInfo + Default + SQLSchema<'a, &'a str, Value> + Expr<'a, Value>
{
    type Table: SQLTable<'a, Self::TableType, Value>;
    type TableType: SQLSchemaType;
    type ForeignKeys;
    type Type: TryInto<Value>;

    const PRIMARY_KEY: bool = false;
    const NOT_NULL: bool = false;
    const UNIQUE: bool = false;
    const DEFAULT: Option<Self::Type> = None;

    fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
        None::<fn() -> Self::Type>
    }

    /// Creates a typed named placeholder for this column's SQL type.
    fn placeholder(
        &self,
        name: &'static str,
    ) -> crate::placeholder::TypedPlaceholder<
        <Self as Expr<'a, Value>>::SQLType,
        <Self as Expr<'a, Value>>::Nullable,
    >
    where
        Self: Sized,
    {
        crate::placeholder::TypedPlaceholder::named(name)
    }
}

// Blanket implementation for references.
impl<T: SQLColumnInfo> SQLColumnInfo for &T {
    fn is_not_null(&self) -> bool {
        (*self).is_not_null()
    }

    fn is_primary_key(&self) -> bool {
        (*self).is_primary_key()
    }

    fn is_unique(&self) -> bool {
        (*self).is_unique()
    }

    fn name(&self) -> &'static str {
        (*self).name()
    }

    fn r#type(&self) -> &'static str {
        (*self).r#type()
    }

    fn has_default(&self) -> bool {
        (*self).has_default()
    }

    fn table(&self) -> &'static dyn SQLTableInfo {
        (*self).table()
    }
}

impl<'a, Value, T> SQLColumn<'a, Value> for &T
where
    Value: SQLParam + 'a,
    T: SQLColumn<'a, Value>,
    for<'r> &'r T: SQLColumnInfo + Default + SQLSchema<'a, &'a str, Value> + Expr<'a, Value>,
{
    type Table = T::Table;
    type TableType = T::TableType;
    type ForeignKeys = T::ForeignKeys;
    type Type = T::Type;

    const PRIMARY_KEY: bool = T::PRIMARY_KEY;
    const NOT_NULL: bool = T::NOT_NULL;
    const UNIQUE: bool = T::UNIQUE;
    const DEFAULT: Option<Self::Type> = T::DEFAULT;

    fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
        <T as SQLColumn<'a, Value>>::default_fn(*self)
    }
}

impl core::fmt::Debug for dyn SQLColumnInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &self.table().name())
            .field("has_default", &self.has_default())
            .finish()
    }
}
