use crate::relation::SchemaHasTable;
use crate::traits::{
    SQLForeignKey, SQLForeignKeyInfo, SQLPrimaryKeyInfo, SQLTableInfo, type_set::Cons,
    type_set::Nil,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SQLConstraintKind {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check,
}

pub trait SQLConstraintInfo: Send + Sync + 'static {
    fn table(&self) -> &'static dyn SQLTableInfo;
    fn name(&self) -> Option<&'static str> {
        None
    }
    fn kind(&self) -> SQLConstraintKind;
    fn columns(&self) -> &'static [&'static str];
    fn primary_key(&self) -> Option<&'static dyn SQLPrimaryKeyInfo> {
        None
    }
    fn foreign_key(&self) -> Option<&'static dyn SQLForeignKeyInfo> {
        None
    }
    fn check_expression(&self) -> Option<&'static str> {
        None
    }
}

pub trait SQLConstraint: SQLConstraintInfo {
    type Table;
    type Kind;
    type Columns;
}

pub struct NoConstraint;

impl SQLConstraintInfo for NoConstraint {
    fn table(&self) -> &'static dyn SQLTableInfo {
        panic!("NoConstraint has no table")
    }

    fn kind(&self) -> SQLConstraintKind {
        SQLConstraintKind::Check
    }

    fn columns(&self) -> &'static [&'static str] {
        &[]
    }
}

impl SQLConstraint for NoConstraint {
    type Table = ();
    type Kind = ();
    type Columns = ();
}

pub struct PrimaryKeyK;
pub struct ForeignKeyK;
pub struct UniqueK;
pub struct CheckK;

#[diagnostic::on_unimplemented(
    message = "table `{Self}` does not have a primary key",
    label = "add `#[column(primary)]` to a column in this table",
    note = "tables used in this context must have a primary key defined"
)]
pub trait HasPrimaryKey {}

#[diagnostic::on_unimplemented(
    message = "table `{Self}` does not have a `{Kind}` constraint",
    label = "this table is missing the required constraint"
)]
pub trait HasConstraint<Kind> {}

#[diagnostic::on_unimplemented(
    message = "foreign key type mismatch: `{Self}` cannot reference column of type `{T}`",
    label = "change this column's type to `{T}` to match the referenced column",
    note = "the foreign key column type must exactly match the referenced column type"
)]
pub trait TypeEq<T> {}
impl<T> TypeEq<T> for T {}

#[diagnostic::on_unimplemented(
    message = "column `{Self}` does not belong to table `{Table}`",
    label = "this column is not defined on the target table",
    note = "constraint columns must be defined on the table they reference"
)]
pub trait ColumnOf<Table> {}

#[diagnostic::on_unimplemented(
    message = "column `{Self}` is nullable and cannot be used here",
    label = "this column must be NOT NULL",
    note = "primary key columns cannot be nullable"
)]
pub trait ColumnNotNull {}

pub trait ColumnValueType {
    type ValueType;
}

#[diagnostic::on_unimplemented(
    message = "one or more columns do not belong to table `{Table}`",
    label = "these columns are not all defined on the same table",
    note = "all constraint columns must belong to the target table"
)]
pub trait ColumnsBelongTo<Table, Cols> {}
impl<Table> ColumnsBelongTo<Table, ()> for () {}

macro_rules! impl_columns_belong_to_cb {
    ($($C:ident),+) => {
        impl<Table, $($C),+> ColumnsBelongTo<Table, ($($C,)+)> for ()
        where $($C: ColumnOf<Table>,)+ {}
    };
}
with_tuple_sizes!(impl_columns_belong_to_cb);

#[diagnostic::on_unimplemented(
    message = "constraint requires at least one column",
    label = "provide one or more columns for this constraint"
)]
pub trait NonEmptyColSet<Cols> {}

macro_rules! impl_non_empty_col_set_cb {
    ($($C:ident),+) => {
        impl<$($C),+> NonEmptyColSet<($($C,)+)> for () {}
    };
}
with_tuple_sizes!(impl_non_empty_col_set_cb);

#[diagnostic::on_unimplemented(
    message = "constraint columns contain duplicates",
    label = "each column must appear only once in a constraint"
)]
pub trait NoDuplicateColSet<Cols> {}
impl NoDuplicateColSet<()> for () {}

#[diagnostic::on_unimplemented(
    message = "primary key columns must all be NOT NULL",
    label = "one or more primary key columns are nullable — remove `Option<>` from them",
    note = "wrap the column type directly (e.g., `pub id: i64`) instead of `Option<i64>`"
)]
pub trait PkNotNull<Cols> {}
impl PkNotNull<()> for () {}

macro_rules! impl_pk_not_null_cb {
    ($($C:ident),+) => {
        impl<$($C),+> PkNotNull<($($C,)+)> for ()
        where $($C: ColumnNotNull,)+ {}
    };
}
with_tuple_sizes!(impl_pk_not_null_cb);

#[diagnostic::on_unimplemented(
    message = "foreign key column count mismatch between source and target",
    label = "the number of FK columns must match the number of referenced columns",
    note = "e.g., a composite FK with 2 source columns must reference exactly 2 target columns"
)]
pub trait FkArityMatch<SrcCols, DstCols> {}

macro_rules! impl_fk_arity_match_cb {
    ($($S:ident),+; $($D:ident),+) => {
        impl<$($S,)+ $($D,)+> FkArityMatch<($($S,)+), ($($D,)+)> for () {}
    };
}
with_dual_tuple_sizes!(impl_fk_arity_match_cb);

#[diagnostic::on_unimplemented(
    message = "foreign key column types do not match the referenced columns",
    label = "each FK column's type must match the corresponding referenced column's type"
)]
pub trait FkTypeMatch<SrcCols, DstCols> {}
impl FkTypeMatch<(), ()> for () {}

macro_rules! impl_fk_type_match_cb {
    ($($S:ident),+; $($D:ident),+) => {
        impl<$($S,)+ $($D,)+> FkTypeMatch<($($S,)+), ($($D,)+)> for ()
        where
            $(
                $S: ColumnValueType,
                $D: ColumnValueType,
                <$S as ColumnValueType>::ValueType: TypeEq<<$D as ColumnValueType>::ValueType>,
            )+
        {}
    };
}
with_dual_tuple_sizes!(impl_fk_type_match_cb);

#[diagnostic::on_unimplemented(
    message = "foreign key references a table not present in this schema",
    label = "add the referenced table to your schema definition",
    note = "all tables referenced by foreign keys must be included in the schema"
)]
pub trait ForeignKeysInSchema<S> {}
impl<S> ForeignKeysInSchema<S> for () {}

macro_rules! impl_foreign_keys_in_schema_cb {
    ($($Fk:ident),+) => {
        impl<S, $($Fk),+> ForeignKeysInSchema<S> for ($($Fk,)+)
        where
            $(
                $Fk: SQLForeignKey,
                S: SchemaHasTable<<$Fk as SQLForeignKey>::TargetTable>,
            )+
        {}
    };
}
with_tuple_sizes!(impl_foreign_keys_in_schema_cb);

#[diagnostic::on_unimplemented(
    message = "schema contains tables with foreign keys referencing tables not in the schema",
    label = "ensure all FK target tables are included in this schema"
)]
pub trait ValidateTableSetForeignKeys<S> {}
impl<S> ValidateTableSetForeignKeys<S> for Nil {}

pub trait SQLTableMeta {
    type ForeignKeys;
    type PrimaryKey;
    type Constraints;
}

impl<S, Head, Tail> ValidateTableSetForeignKeys<S> for Cons<Head, Tail>
where
    Head: SQLTableMeta,
    <Head as SQLTableMeta>::ForeignKeys: ForeignKeysInSchema<S>,
    Tail: ValidateTableSetForeignKeys<S>,
{
}

pub trait ValidateSchemaItemForeignKeys<S> {}

impl<S, Item> ValidateSchemaItemForeignKeys<S> for Item
where
    Item: crate::traits::SchemaItemTables,
    <Item as crate::traits::SchemaItemTables>::Tables: ValidateTableSetForeignKeys<S>,
{
}

/// Marker trait for valid ON CONFLICT column targets.
/// Generated for PK columns, unique columns, PK ZSTs, unique constraint ZSTs,
/// and unique index structs.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid conflict target for table `{Table}`",
    label = "use a primary key, unique column, or unique index/constraint from this table",
    note = "ON CONFLICT targets must be primary key columns, unique columns, or unique indexes"
)]
pub trait ConflictTarget<Table> {
    fn conflict_columns(&self) -> &'static [&'static str];
}

/// Marker trait for named constraints usable with ON CONFLICT ON CONSTRAINT (PG-only).
/// Generated for unique index structs and unique constraint ZSTs that have a name.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a named constraint on table `{Table}`",
    label = "ON CONSTRAINT requires a named unique index or unique constraint"
)]
pub trait NamedConstraint<Table> {
    fn constraint_name(&self) -> &'static str;
}
