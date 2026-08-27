//! Typed derived-table projections.
//!
//! A [`Derived`] value owns a complete query and exposes only the fields named
//! by its projection. Dialect builders construct these values after checking
//! that the query is in an executable state.

use core::marker::PhantomData;

use crate::expr::{AllScalar, Expr, HasAggStatus, Scalar};
use crate::row::{
    ExprValueType, GroupByIdentity, HasSelectModel, IntoGroupBy, IntoSelectTarget, Scoped,
    SelectCols, SelectStar,
};
use crate::{Cons, Nil, SQL, SQLColumnInfo, SQLParam, SQLSchemaType, SQLTable, Tag, ToSQL, Token};

mod private {
    pub trait Projection {}
    pub trait Selection<'a, V: crate::SQLParam, Schema, Table> {}
    pub trait Output {}
}

/// A complete query used as a named source in another query.
pub struct Derived<'a, V: SQLParam, Name, Projection, Query> {
    query: Query,
    marker: PhantomData<(&'a (), V, Name, Projection)>,
}

impl<'a, V, Name, Projection, Query> Derived<'a, V, Name, Projection, Query>
where
    V: SQLParam,
    Name: Tag,
    Projection: DerivedProjection<Name>,
{
    /// Constructs a derived source without validating its query.
    ///
    /// # Safety
    ///
    /// `query` must select exactly the columns described by `Projection`, in
    /// the same order, and must already satisfy the dialect builder's scope
    /// and aggregate rules.
    #[doc(hidden)]
    #[track_caller]
    pub unsafe fn new_unchecked(query: Query) -> Self {
        Projection::validate();
        Self {
            query,
            marker: PhantomData,
        }
    }

    /// Returns the underlying query.
    pub const fn query(&self) -> &Query {
        &self.query
    }

    /// Returns the underlying query by value.
    pub fn into_query(self) -> Query {
        self.query
    }

    /// Returns typed fields qualified by this source's name.
    pub fn fields(&self) -> Projection::Fields
    where
        Name: Tag,
        Projection: DerivedProjection<Name>,
    {
        Projection::fields()
    }
}

impl<'a, V: SQLParam, Name, Projection, Query: Clone> Clone
    for Derived<'a, V, Name, Projection, Query>
{
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            marker: PhantomData,
        }
    }
}

impl<'a, V, Name, Projection, Query> ToSQL<'a, V> for Derived<'a, V, Name, Projection, Query>
where
    V: SQLParam,
    Name: Tag,
    Query: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.query.to_sql().parens().alias(Name::NAME)
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.query.into_sql().parens().alias(Name::NAME)
    }
}

impl<'a, V, Name, Projection, Query> HasSelectModel for Derived<'a, V, Name, Projection, Query>
where
    V: SQLParam,
    Name: Tag,
    Projection: DerivedProjection<Name>,
{
    type SelectModel = Projection::Row;

    const COLUMN_COUNT: usize = Projection::COLUMN_COUNT;
}

/// Maps a SELECT projection to the fields and row exposed by a derived source.
#[doc(hidden)]
pub trait DerivedProjection<Name: Tag>: private::Projection {
    type Fields;
    type Row;

    const COLUMN_COUNT: usize;

    fn validate() {}

    fn fields() -> Self::Fields;
}

/// Select-marker capability for becoming a derived source.
///
/// The single-table `SELECT *` implementation deliberately matches only an
/// exact one-table scope. After a join, `SELECT *` contains more columns and
/// cannot soundly expose the last joined table as its complete projection.
#[doc(hidden)]
pub trait DerivedSelection<'a, V: SQLParam, Schema, Table>:
    private::Selection<'a, V, Schema, Table>
{
    type Projection;
}

impl<'a, V, Schema, Table> DerivedSelection<'a, V, Schema, Table>
    for Scoped<SelectStar, Cons<Table, Nil>>
where
    V: SQLParam + 'a,
    Schema: SQLSchemaType,
    Table: SQLTable<'a, Schema, V>,
{
    type Projection = TableProjection<'a, V, Schema, Table>;
}

impl<'a, V, Schema, Table> private::Selection<'a, V, Schema, Table>
    for Scoped<SelectStar, Cons<Table, Nil>>
where
    V: SQLParam + 'a,
    Schema: SQLSchemaType,
    Table: SQLTable<'a, Schema, V>,
{
}

impl<'a, V, Schema, Name, Projection, Query>
    DerivedSelection<'a, V, Schema, Derived<'a, V, Name, Projection, Query>>
    for Scoped<SelectStar, Cons<Derived<'a, V, Name, Projection, Query>, Nil>>
where
    V: SQLParam,
    Name: Tag,
    Projection: DerivedProjection<Name>,
{
    type Projection = Projection;
}

impl<'a, V, Schema, Name, Projection, Query>
    private::Selection<'a, V, Schema, Derived<'a, V, Name, Projection, Query>>
    for Scoped<SelectStar, Cons<Derived<'a, V, Name, Projection, Query>, Nil>>
where
    V: SQLParam,
    Name: Tag,
    Projection: DerivedProjection<Name>,
{
}

impl<'a, V, Schema, Table, Columns, Scope> DerivedSelection<'a, V, Schema, Table>
    for Scoped<SelectCols<Columns>, Scope>
where
    V: SQLParam,
{
    type Projection = Self;
}

impl<'a, V, Schema, Table, Columns, Scope> private::Selection<'a, V, Schema, Table>
    for Scoped<SelectCols<Columns>, Scope>
where
    V: SQLParam,
{
}

impl<Name, Marker, Scope> DerivedProjection<Name> for Scoped<Marker, Scope>
where
    Name: Tag,
    Marker: DerivedProjection<Name>,
{
    type Fields = Marker::Fields;
    type Row = Marker::Row;

    const COLUMN_COUNT: usize = Marker::COLUMN_COUNT;

    fn validate() {
        Marker::validate();
    }

    fn fields() -> Self::Fields {
        Marker::fields()
    }
}

impl<Marker, Scope> private::Projection for Scoped<Marker, Scope> where Marker: private::Projection {}

/// Projection marker used when a dialect has proven that `SELECT *` comes from
/// one base table.
#[doc(hidden)]
pub struct TableProjection<'a, V: SQLParam, Schema, Table>(PhantomData<(&'a (), V, Schema, Table)>);

impl<V: SQLParam, Schema, Table> private::Projection for TableProjection<'_, V, Schema, Table> {}

impl<'a, V, Schema, Name, Table> DerivedProjection<Name> for TableProjection<'a, V, Schema, Table>
where
    V: SQLParam + 'a,
    Schema: SQLSchemaType,
    Name: Tag + 'static,
    Table: SQLTable<'a, Schema, V> + HasSelectModel,
    Table::Aliased<Name>: HasSelectModel<SelectModel = Table::SelectModel>,
{
    type Fields = Table::Aliased<Name>;
    type Row = Table::SelectModel;

    const COLUMN_COUNT: usize = Table::COLUMN_COUNT;

    fn fields() -> Self::Fields {
        Table::alias::<Name>()
    }
}

/// A field exposed by a named derived source.
pub struct DerivedField<Name, Output>(PhantomData<(Name, Output)>);

impl<Name, Output> Copy for DerivedField<Name, Output> {}

impl<Name, Output> Clone for DerivedField<Name, Output> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Name, Output> Default for DerivedField<Name, Output> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'a, V, Name, Output> ToSQL<'a, V> for DerivedField<Name, Output>
where
    V: SQLParam,
    Name: Tag,
    Output: ProjectionOutput,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::ident(Name::NAME)
            .push(Token::DOT)
            .append(SQL::ident(Output::output_name()))
    }
}

impl<'a, V, Name, Output> Expr<'a, V> for DerivedField<Name, Output>
where
    V: SQLParam + 'a,
    Name: Tag,
    Output: ProjectionOutput + Expr<'a, V>,
{
    type SQLType = Output::SQLType;
    type Nullable = Output::Nullable;
    type Aggregate = Scalar;
}

impl<Name, Output> ExprValueType for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput + ExprValueType,
{
    type ValueType = Output::ValueType;
}

impl<Name, Output> IntoSelectTarget for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput + ExprValueType,
{
    type Marker = SelectCols<(Self,)>;
}

impl<Name, Output> HasAggStatus for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput,
{
    type Status = AllScalar;
}

impl<Name, Output> GroupByIdentity for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput,
{
    type Identity = Self;
}

impl<'a, V, Name, Output> IntoGroupBy<'a, V> for DerivedField<Name, Output>
where
    V: SQLParam + 'a,
    Name: Tag,
    Output: ProjectionOutput,
{
    type Columns = Cons<Self, Nil>;
}

/// Supplies the static output name for one SELECT expression.
#[doc(hidden)]
pub trait ProjectionOutput: private::Output {
    fn output_name() -> &'static str;
}

impl<Column> private::Output for Column where Column: SQLColumnInfo + Default {}

impl<Column> ProjectionOutput for Column
where
    Column: SQLColumnInfo + Default,
{
    fn output_name() -> &'static str {
        Column::default().name()
    }
}

impl<E, Name> ProjectionOutput for crate::expr::NamedExpr<E, Name>
where
    Name: Tag,
{
    fn output_name() -> &'static str {
        Name::NAME
    }
}

impl<E, Name> private::Output for crate::expr::NamedExpr<E, Name> where Name: Tag {}

impl<Name, Output> ProjectionOutput for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput,
{
    fn output_name() -> &'static str {
        Output::output_name()
    }
}

impl<Name, Output> private::Output for DerivedField<Name, Output>
where
    Name: Tag,
    Output: ProjectionOutput,
{
}

macro_rules! impl_derived_projection_tuple {
    ($($output:ident),+; $($_index:tt),+) => {
        impl<Name, $($output),+> DerivedProjection<Name> for SelectCols<($($output,)+)>
        where
            Name: Tag,
            $($output: ProjectionOutput + ExprValueType,)+
        {
            type Fields = ($(DerivedField<Name, $output>,)+);
            type Row = ($(<$output as ExprValueType>::ValueType,)+);

            const COLUMN_COUNT: usize = impl_derived_projection_tuple!(@count $($output),+);

            fn validate() {
                let names = [$(<$output as ProjectionOutput>::output_name(),)+];
                let mut left = 0;
                while left < names.len() {
                    let mut right = left + 1;
                    while right < names.len() {
                        assert!(
                            names[left] != names[right],
                            "derived projection contains duplicate output name `{}`; name one expression with `.named::<Tag>()`",
                            names[left],
                        );
                        right += 1;
                    }
                    left += 1;
                }
            }

            fn fields() -> Self::Fields {
                ($(DerivedField::<Name, $output>::default(),)+)
            }
        }

        impl<$($output),+> private::Projection for SelectCols<($($output,)+)>
        where
            $($output: ProjectionOutput + ExprValueType,)+
        {
        }
    };
    (@count $head:ident $(,$tail:ident)*) => {
        1usize $(+ { let _ = stringify!($tail); 1usize })*
    };
}

with_col_sizes_8!(impl_derived_projection_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_derived_projection_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_derived_projection_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_derived_projection_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_derived_projection_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_derived_projection_tuple);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dialect, SQLiteDialect};

    #[derive(Clone, Debug)]
    struct TestParam;

    impl SQLParam for TestParam {
        const DIALECT: Dialect = Dialect::SQLite;
        type DialectMarker = SQLiteDialect;
    }

    struct Alias;

    impl Tag for Alias {
        const NAME: &'static str = "alias";
    }

    struct First;
    struct Second;

    impl private::Output for First {}
    impl private::Output for Second {}

    impl ProjectionOutput for First {
        fn output_name() -> &'static str {
            "duplicate"
        }
    }

    impl ProjectionOutput for Second {
        fn output_name() -> &'static str {
            "duplicate"
        }
    }

    impl ExprValueType for First {
        type ValueType = i32;
    }

    impl ExprValueType for Second {
        type ValueType = i32;
    }

    #[test]
    #[should_panic(expected = "duplicate output name")]
    fn duplicate_projection_names_are_rejected() {
        let _: Derived<'_, TestParam, Alias, SelectCols<(First, Second)>, ()> =
            // SAFETY: This test exercises projection-name validation before
            // the query is ever rendered or decoded.
            unsafe { Derived::new_unchecked(()) };
    }
}
