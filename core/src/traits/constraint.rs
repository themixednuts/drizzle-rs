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

pub trait HasPrimaryKey {}
pub trait HasConstraint<Kind> {}
pub trait TypeEq<T> {}
impl<T> TypeEq<T> for T {}

pub trait ColumnOf<Table> {}
pub trait ColumnNotNull {}
pub trait ColumnValueType {
    type ValueType;
}

pub trait ColumnsBelongTo<Table, Cols> {}
impl<Table> ColumnsBelongTo<Table, ()> for () {}

macro_rules! impl_columns_belong_to {
    ($(($($C:ident),+)),+ $(,)?) => {
        $(
            impl<Table, $($C),+> ColumnsBelongTo<Table, ($($C,)+)> for ()
            where
                $($C: ColumnOf<Table>,)+
            {}
        )+
    };
}

impl_columns_belong_to!(
    (C0),
    (C0, C1),
    (C0, C1, C2),
    (C0, C1, C2, C3),
    (C0, C1, C2, C3, C4),
    (C0, C1, C2, C3, C4, C5),
    (C0, C1, C2, C3, C4, C5, C6),
    (C0, C1, C2, C3, C4, C5, C6, C7),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14
    ),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
    )
);

pub trait NonEmptyColSet<Cols> {}

macro_rules! impl_non_empty_col_set {
    ($(($($C:ident),+)),+ $(,)?) => {
        $(
            impl<$($C),+> NonEmptyColSet<($($C,)+)> for () {}
        )+
    };
}

impl_non_empty_col_set!(
    (C0),
    (C0, C1),
    (C0, C1, C2),
    (C0, C1, C2, C3),
    (C0, C1, C2, C3, C4),
    (C0, C1, C2, C3, C4, C5),
    (C0, C1, C2, C3, C4, C5, C6),
    (C0, C1, C2, C3, C4, C5, C6, C7),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14
    ),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
    )
);

pub trait NoDuplicateColSet<Cols> {}
impl NoDuplicateColSet<()> for () {}

pub trait PkNotNull<Cols> {}
impl PkNotNull<()> for () {}

macro_rules! impl_pk_not_null {
    ($(($($C:ident),+)),+ $(,)?) => {
        $(
            impl<$($C),+> PkNotNull<($($C,)+)> for ()
            where
                $($C: ColumnNotNull,)+
            {}
        )+
    };
}

impl_pk_not_null!(
    (C0),
    (C0, C1),
    (C0, C1, C2),
    (C0, C1, C2, C3),
    (C0, C1, C2, C3, C4),
    (C0, C1, C2, C3, C4, C5),
    (C0, C1, C2, C3, C4, C5, C6),
    (C0, C1, C2, C3, C4, C5, C6, C7),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12),
    (C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14
    ),
    (
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
    )
);

pub trait FkArityMatch<SrcCols, DstCols> {}

macro_rules! impl_fk_arity_match {
    ($(($($S:ident),+) => ($($D:ident),+)),+ $(,)?) => {
        $(
            impl<$($S,)+ $($D,)+> FkArityMatch<($($S,)+), ($($D,)+)> for () {}
        )+
    };
}

impl_fk_arity_match!(
    (S0) => (D0),
    (S0, S1) => (D0, D1),
    (S0, S1, S2) => (D0, D1, D2),
    (S0, S1, S2, S3) => (D0, D1, D2, D3),
    (S0, S1, S2, S3, S4) => (D0, D1, D2, D3, D4),
    (S0, S1, S2, S3, S4, S5) => (D0, D1, D2, D3, D4, D5),
    (S0, S1, S2, S3, S4, S5, S6) => (D0, D1, D2, D3, D4, D5, D6),
    (S0, S1, S2, S3, S4, S5, S6, S7) => (D0, D1, D2, D3, D4, D5, D6, D7),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8) => (D0, D1, D2, D3, D4, D5, D6, D7, D8),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14, S15) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14, D15)
);

pub trait FkTypeMatch<SrcCols, DstCols> {}
impl FkTypeMatch<(), ()> for () {}

macro_rules! impl_fk_type_match {
    ($(($($S:ident),+) => ($($D:ident),+)),+ $(,)?) => {
        $(
            impl<$($S,)+ $($D,)+> FkTypeMatch<($($S,)+), ($($D,)+)> for ()
            where
                $(
                    $S: ColumnValueType,
                    $D: ColumnValueType,
                    <$S as ColumnValueType>::ValueType: TypeEq<<$D as ColumnValueType>::ValueType>,
                )+
            {}
        )+
    };
}

impl_fk_type_match!(
    (S0) => (D0),
    (S0, S1) => (D0, D1),
    (S0, S1, S2) => (D0, D1, D2),
    (S0, S1, S2, S3) => (D0, D1, D2, D3),
    (S0, S1, S2, S3, S4) => (D0, D1, D2, D3, D4),
    (S0, S1, S2, S3, S4, S5) => (D0, D1, D2, D3, D4, D5),
    (S0, S1, S2, S3, S4, S5, S6) => (D0, D1, D2, D3, D4, D5, D6),
    (S0, S1, S2, S3, S4, S5, S6, S7) => (D0, D1, D2, D3, D4, D5, D6, D7),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8) => (D0, D1, D2, D3, D4, D5, D6, D7, D8),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14),
    (S0, S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14, S15) => (D0, D1, D2, D3, D4, D5, D6, D7, D8, D9, D10, D11, D12, D13, D14, D15)
);

pub trait ForeignKeysInSchema<S> {}
impl<S> ForeignKeysInSchema<S> for () {}

macro_rules! impl_foreign_keys_in_schema {
    ($($fk:ident),+ $(,)?) => {
        impl<S, $($fk),+> ForeignKeysInSchema<S> for ($($fk,)+)
        where
            $(
                $fk: SQLForeignKey,
                S: SchemaHasTable<<$fk as SQLForeignKey>::TargetTable>,
            )+
        {}
    };
}

impl_foreign_keys_in_schema!(Fk0);
impl_foreign_keys_in_schema!(Fk0, Fk1);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10);
impl_foreign_keys_in_schema!(Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10, Fk11);
impl_foreign_keys_in_schema!(
    Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10, Fk11, Fk12
);
impl_foreign_keys_in_schema!(
    Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10, Fk11, Fk12, Fk13
);
impl_foreign_keys_in_schema!(
    Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10, Fk11, Fk12, Fk13, Fk14
);
impl_foreign_keys_in_schema!(
    Fk0, Fk1, Fk2, Fk3, Fk4, Fk5, Fk6, Fk7, Fk8, Fk9, Fk10, Fk11, Fk12, Fk13, Fk14, Fk15
);

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
pub trait ConflictTarget<Table> {
    fn conflict_columns(&self) -> &'static [&'static str];
}

/// Marker trait for named constraints usable with ON CONFLICT ON CONSTRAINT (PG-only).
/// Generated for unique index structs and unique constraint ZSTs that have a name.
pub trait NamedConstraint<Table> {
    fn constraint_name(&self) -> &'static str;
}
