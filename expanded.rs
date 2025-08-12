#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2024::*;
#[macro_use]
extern crate std;
use common::setup_db;
use drizzle_core::SQL;
use drizzle_core::SQLTableInfo;
use drizzle_rs::prelude::*;
use drizzle_rs::{core::eq, sqlite::{SQLiteValue, params}};
use procmacros::{SQLiteTable, drizzle};
use rusqlite::Row;
use crate::common::Complex;
pub struct Simple {
    pub id: SimpleId,
    pub name: SimpleName,
}
#[automatically_derived]
impl ::core::default::Default for Simple {
    #[inline]
    fn default() -> Simple {
        Simple {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
        }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for Simple {
    #[inline]
    fn clone(&self) -> Simple {
        let _: ::core::clone::AssertParamIsClone<SimpleId>;
        let _: ::core::clone::AssertParamIsClone<SimpleName>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for Simple {}
#[automatically_derived]
impl ::core::fmt::Debug for Simple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "Simple",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Simple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Simple {
    #[inline]
    fn eq(&self, other: &Simple) -> bool {
        self.id == other.id && self.name == other.name
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Simple {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {
        let _: ::core::cmp::AssertParamIsEq<SimpleId>;
        let _: ::core::cmp::AssertParamIsEq<SimpleName>;
    }
}
#[automatically_derived]
impl ::core::hash::Hash for Simple {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
        ::core::hash::Hash::hash(&self.id, state);
        ::core::hash::Hash::hash(&self.name, state)
    }
}
#[automatically_derived]
impl ::core::cmp::PartialOrd for Simple {
    #[inline]
    fn partial_cmp(
        &self,
        other: &Simple,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        match ::core::cmp::PartialOrd::partial_cmp(&self.id, &other.id) {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                ::core::cmp::PartialOrd::partial_cmp(&self.name, &other.name)
            }
            cmp => cmp,
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Ord for Simple {
    #[inline]
    fn cmp(&self, other: &Simple) -> ::core::cmp::Ordering {
        match ::core::cmp::Ord::cmp(&self.id, &other.id) {
            ::core::cmp::Ordering::Equal => {
                ::core::cmp::Ord::cmp(&self.name, &other.name)
            }
            cmp => cmp,
        }
    }
}
#[allow(non_upper_case_globals)]
impl Simple {
    const fn new() -> Self {
        Self {
            id: SimpleId::new(),
            name: SimpleName::new(),
        }
    }
    pub const id: SimpleId = SimpleId;
    pub const name: SimpleName = SimpleName;
}
#[allow(non_camel_case_types)]
pub struct SimpleId;
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::fmt::Debug for SimpleId {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "SimpleId")
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::clone::Clone for SimpleId {
    #[inline]
    fn clone(&self) -> SimpleId {
        *self
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::marker::Copy for SimpleId {}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::default::Default for SimpleId {
    #[inline]
    fn default() -> SimpleId {
        SimpleId {}
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialOrd for SimpleId {
    #[inline]
    fn partial_cmp(
        &self,
        other: &SimpleId,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::Ord for SimpleId {
    #[inline]
    fn cmp(&self, other: &SimpleId) -> ::core::cmp::Ordering {
        ::core::cmp::Ordering::Equal
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::Eq for SimpleId {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {}
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::marker::StructuralPartialEq for SimpleId {}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialEq for SimpleId {
    #[inline]
    fn eq(&self, other: &SimpleId) -> bool {
        true
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::hash::Hash for SimpleId {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
}
impl SimpleId {
    const fn new() -> SimpleId {
        SimpleId {}
    }
}
impl<
    'a,
> ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleId {
    const NAME: &'a str = "id";
    const TYPE: &'a str = "INTEGER";
    const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
        "id INTEGER PRIMARY KEY NOT NULL",
    );
}
impl ::drizzle_rs::core::SQLColumnInfo for SimpleId {
    fn name(&self) -> &str {
        Self::NAME
    }
    fn r#type(&self) -> &str {
        Self::TYPE
    }
    fn is_primary_key(&self) -> bool {
        Self::PRIMARY_KEY
    }
    fn is_not_null(&self) -> bool {
        Self::NOT_NULL
    }
    fn is_unique(&self) -> bool {
        Self::UNIQUE
    }
    fn has_default(&self) -> bool {
        false
    }
    fn table(&self) -> &dyn SQLTableInfo {
        static TABLE: Simple = Simple::new();
        &TABLE
    }
}
impl ::drizzle_rs::sqlite::SQLiteColumnInfo for SimpleId {
    fn is_autoincrement(&self) -> bool {
        <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
    }
}
impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleId {
    type Table = Simple;
    type Type = i32;
    const PRIMARY_KEY: bool = true;
    const NOT_NULL: bool = true;
    const UNIQUE: bool = false;
    const DEFAULT: Option<Self::Type> = None;
    fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
        None::<fn() -> Self::Type>
    }
}
impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for SimpleId {
    const AUTOINCREMENT: bool = false;
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleId {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        use ::drizzle_rs::core::ToSQL;
        static INSTANCE: SimpleId = SimpleId::new();
        INSTANCE.as_column().to_sql()
    }
}
impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for SimpleId {
    fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
        ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("id"))
    }
}
#[allow(non_camel_case_types)]
pub struct SimpleName;
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::fmt::Debug for SimpleName {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "SimpleName")
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::clone::Clone for SimpleName {
    #[inline]
    fn clone(&self) -> SimpleName {
        *self
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::marker::Copy for SimpleName {}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::default::Default for SimpleName {
    #[inline]
    fn default() -> SimpleName {
        SimpleName {}
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialOrd for SimpleName {
    #[inline]
    fn partial_cmp(
        &self,
        other: &SimpleName,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(::core::cmp::Ordering::Equal)
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::Ord for SimpleName {
    #[inline]
    fn cmp(&self, other: &SimpleName) -> ::core::cmp::Ordering {
        ::core::cmp::Ordering::Equal
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::Eq for SimpleName {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {}
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::marker::StructuralPartialEq for SimpleName {}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialEq for SimpleName {
    #[inline]
    fn eq(&self, other: &SimpleName) -> bool {
        true
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::hash::Hash for SimpleName {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
}
impl SimpleName {
    const fn new() -> SimpleName {
        SimpleName {}
    }
}
impl<
    'a,
> ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleName {
    const NAME: &'a str = "name";
    const TYPE: &'a str = "TEXT";
    const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
        "name TEXT NOT NULL",
    );
}
impl ::drizzle_rs::core::SQLColumnInfo for SimpleName {
    fn name(&self) -> &str {
        Self::NAME
    }
    fn r#type(&self) -> &str {
        Self::TYPE
    }
    fn is_primary_key(&self) -> bool {
        Self::PRIMARY_KEY
    }
    fn is_not_null(&self) -> bool {
        Self::NOT_NULL
    }
    fn is_unique(&self) -> bool {
        Self::UNIQUE
    }
    fn has_default(&self) -> bool {
        false
    }
    fn table(&self) -> &dyn SQLTableInfo {
        static TABLE: Simple = Simple::new();
        &TABLE
    }
}
impl ::drizzle_rs::sqlite::SQLiteColumnInfo for SimpleName {
    fn is_autoincrement(&self) -> bool {
        <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
    }
}
impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleName {
    type Table = Simple;
    type Type = String;
    const PRIMARY_KEY: bool = false;
    const NOT_NULL: bool = true;
    const UNIQUE: bool = false;
    const DEFAULT: Option<Self::Type> = None;
    fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
        None::<fn() -> Self::Type>
    }
}
impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for SimpleName {
    const AUTOINCREMENT: bool = false;
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SimpleName {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        use ::drizzle_rs::core::ToSQL;
        static INSTANCE: SimpleName = SimpleName::new();
        INSTANCE.as_column().to_sql()
    }
}
impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for SimpleName {
    fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
        ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("name"))
    }
}
impl<
    'a,
> ::drizzle_rs::core::SQLSchema<
    'a,
    ::drizzle_rs::core::SQLSchemaType,
    ::drizzle_rs::sqlite::SQLiteValue<'a>,
> for Simple {
    const NAME: &'a str = "simple";
    const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
    const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
        "CREATE TABLE \"simple\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL);",
    );
}
impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for Simple {
    type Select = SelectSimple;
    type Insert = InsertSimple;
    type Update = UpdateSimple;
}
impl ::drizzle_rs::core::SQLTableInfo for Simple {
    fn name(&self) -> &str {
        Self::NAME
    }
    fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
        Self::TYPE
    }
    fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
        #[allow(non_upper_case_globals)]
        static SimpleId: SimpleId = SimpleId::new();
        #[allow(non_upper_case_globals)]
        static SimpleName: SimpleName = SimpleName::new();
        Box::new([SimpleId.as_column(), SimpleName.as_column()])
    }
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for Simple {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        use ::drizzle_rs::core::ToSQL;
        static INSTANCE: Simple = Simple::new();
        INSTANCE.as_table().to_sql()
    }
}
pub struct SelectSimple {
    pub id: i32,
    pub name: String,
}
#[automatically_derived]
impl ::core::fmt::Debug for SelectSimple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "SelectSimple",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for SelectSimple {
    #[inline]
    fn clone(&self) -> SelectSimple {
        SelectSimple {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for SelectSimple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for SelectSimple {
    #[inline]
    fn eq(&self, other: &SelectSimple) -> bool {
        self.id == other.id && self.name == other.name
    }
}
#[automatically_derived]
impl ::core::default::Default for SelectSimple {
    #[inline]
    fn default() -> SelectSimple {
        SelectSimple {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
        }
    }
}
pub struct PartialSelectSimple {
    pub id: Option<i32>,
    pub name: Option<String>,
}
#[automatically_derived]
impl ::core::fmt::Debug for PartialSelectSimple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "PartialSelectSimple",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for PartialSelectSimple {
    #[inline]
    fn clone(&self) -> PartialSelectSimple {
        PartialSelectSimple {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for PartialSelectSimple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for PartialSelectSimple {
    #[inline]
    fn eq(&self, other: &PartialSelectSimple) -> bool {
        self.id == other.id && self.name == other.name
    }
}
#[automatically_derived]
impl ::core::default::Default for PartialSelectSimple {
    #[inline]
    fn default() -> PartialSelectSimple {
        PartialSelectSimple {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
        }
    }
}
impl PartialSelectSimple {
    pub fn with_id(mut self, value: i32) -> Self {
        self.id = Some(value);
        self
    }
    pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
        let value = value.into();
        self.name = Some(value);
        self
    }
}
impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SelectSimple {
    type Partial = PartialSelectSimple;
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for PartialSelectSimple {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        ::core::panicking::panic("not implemented")
    }
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SelectSimple {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        ::core::panicking::panic("not implemented")
    }
}
pub struct InsertSimple {
    pub id: ::drizzle_rs::sqlite::InsertValue<i32>,
    pub name: ::drizzle_rs::sqlite::InsertValue<String>,
}
#[automatically_derived]
impl ::core::fmt::Debug for InsertSimple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "InsertSimple",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for InsertSimple {
    #[inline]
    fn clone(&self) -> InsertSimple {
        InsertSimple {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for InsertSimple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for InsertSimple {
    #[inline]
    fn eq(&self, other: &InsertSimple) -> bool {
        self.id == other.id && self.name == other.name
    }
}
impl Default for InsertSimple {
    fn default() -> Self {
        Self {
            id: ::drizzle_rs::sqlite::InsertValue::Omit,
            name: ::drizzle_rs::sqlite::InsertValue::Omit,
        }
    }
}
impl InsertSimple {
    pub fn new(name: impl Into<::std::string::String>) -> Self {
        Self {
            name: ::drizzle_rs::sqlite::InsertValue::Value(name.into()),
            ..Self::default()
        }
    }
    pub fn with_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
        mut self,
        value: V,
    ) -> Self {
        self.id = value.into();
        self
    }
    pub fn with_name<V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>>(
        mut self,
        value: V,
    ) -> Self {
        self.name = value.into();
        self
    }
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for InsertSimple {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        let mut values = Vec::new();
        match &self.id {
            ::drizzle_rs::sqlite::InsertValue::Omit => {}
            ::drizzle_rs::sqlite::InsertValue::Null => {
                values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
            }
            ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
        }
        match &self.name {
            ::drizzle_rs::sqlite::InsertValue::Omit => {}
            ::drizzle_rs::sqlite::InsertValue::Null => {
                values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
            }
            ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
        }
        ::drizzle_rs::core::SQL::parameters(values)
    }
}
impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for InsertSimple {
    fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
        static TABLE: Simple = Simple::new();
        let all_columns = TABLE.columns();
        let mut result_columns = Vec::new();
        if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.id {} else {
            result_columns.push(all_columns[0usize]);
        }
        if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.name {} else {
            result_columns.push(all_columns[1usize]);
        }
        result_columns.into_boxed_slice()
    }
    fn values(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        let mut values = Vec::new();
        match &self.id {
            ::drizzle_rs::sqlite::InsertValue::Omit => {}
            ::drizzle_rs::sqlite::InsertValue::Null => {
                values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
            }
            ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
        }
        match &self.name {
            ::drizzle_rs::sqlite::InsertValue::Omit => {}
            ::drizzle_rs::sqlite::InsertValue::Null => {
                values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
            }
            ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
        }
        ::drizzle_rs::core::SQL::parameters(values)
    }
}
pub struct UpdateSimple {
    pub id: ::std::option::Option<i32>,
    pub name: ::std::option::Option<String>,
}
#[automatically_derived]
impl ::core::fmt::Debug for UpdateSimple {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "UpdateSimple",
            "id",
            &self.id,
            "name",
            &&self.name,
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for UpdateSimple {
    #[inline]
    fn clone(&self) -> UpdateSimple {
        UpdateSimple {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for UpdateSimple {}
#[automatically_derived]
impl ::core::cmp::PartialEq for UpdateSimple {
    #[inline]
    fn eq(&self, other: &UpdateSimple) -> bool {
        self.id == other.id && self.name == other.name
    }
}
#[automatically_derived]
impl ::core::default::Default for UpdateSimple {
    #[inline]
    fn default() -> UpdateSimple {
        UpdateSimple {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
        }
    }
}
impl UpdateSimple {
    pub fn with_id(mut self, value: i32) -> Self {
        self.id = Some(value);
        self
    }
    pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
        let value = value.into();
        self.name = Some(value);
        self
    }
}
impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for UpdateSimple {
    fn to_sql(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        let mut assignments = Vec::new();
        if let Some(val) = &self.id {
            assignments
                .push((
                    "id",
                    val
                        .clone()
                        .try_into()
                        .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                ));
        }
        if let Some(val) = &self.name {
            assignments
                .push((
                    "name",
                    val
                        .clone()
                        .try_into()
                        .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                ));
        }
        ::drizzle_rs::core::SQL::assignments(assignments)
    }
}
impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for SelectSimple {
    fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
        static INSTANCE: Simple = Simple::new();
        INSTANCE.columns()
    }
    fn values(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        ::drizzle_rs::core::SQL::empty()
    }
}
impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for UpdateSimple {
    fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
        static INSTANCE: Simple = Simple::new();
        INSTANCE.columns()
    }
    fn values(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        let mut values = Vec::new();
        if let Some(val) = &self.id {
            values
                .push(
                    val
                        .clone()
                        .try_into()
                        .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                );
        }
        if let Some(val) = &self.name {
            values
                .push(
                    val
                        .clone()
                        .try_into()
                        .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                );
        }
        ::drizzle_rs::core::SQL::parameters(values)
    }
}
impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
for PartialSelectSimple {
    fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
        static INSTANCE: Simple = Simple::new();
        INSTANCE.columns()
    }
    fn values(
        &self,
    ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
        ::drizzle_rs::core::SQL::empty()
    }
}
impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectSimple {
    type Error = ::rusqlite::Error;
    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
}
impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectSimple {
    type Error = ::rusqlite::Error;
    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
}
impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdateSimple {
    type Error = ::rusqlite::Error;
    fn try_from(row: &::rusqlite::Row<'_>) -> ::std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
}
mod common {
    use drizzle_rs::prelude::*;
    use rusqlite::Connection;
    use uuid::Uuid;
    pub struct UserMetadata {
        pub preferences: Vec<String>,
        pub last_login: Option<String>,
        pub theme: String,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for UserMetadata {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "UserMetadata",
                    false as usize + 1 + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "preferences",
                    &self.preferences,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "last_login",
                    &self.last_login,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "theme",
                    &self.theme,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for UserMetadata {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            2u64 => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "preferences" => _serde::__private::Ok(__Field::__field0),
                            "last_login" => _serde::__private::Ok(__Field::__field1),
                            "theme" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"preferences" => _serde::__private::Ok(__Field::__field0),
                            b"last_login" => _serde::__private::Ok(__Field::__field1),
                            b"theme" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<UserMetadata>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = UserMetadata;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct UserMetadata",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            Vec<String>,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct UserMetadata with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            Option<String>,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct UserMetadata with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field2 = match _serde::de::SeqAccess::next_element::<
                            String,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        2usize,
                                        &"struct UserMetadata with 3 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(UserMetadata {
                            preferences: __field0,
                            last_login: __field1,
                            theme: __field2,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<Vec<String>> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<Option<String>> = _serde::__private::None;
                        let mut __field2: _serde::__private::Option<String> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "preferences",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            Vec<String>,
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "last_login",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            Option<String>,
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field2 => {
                                    if _serde::__private::Option::is_some(&__field2) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("theme"),
                                        );
                                    }
                                    __field2 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("preferences")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("last_login")?
                            }
                        };
                        let __field2 = match __field2 {
                            _serde::__private::Some(__field2) => __field2,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("theme")?
                            }
                        };
                        _serde::__private::Ok(UserMetadata {
                            preferences: __field0,
                            last_login: __field1,
                            theme: __field2,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[
                    "preferences",
                    "last_login",
                    "theme",
                ];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "UserMetadata",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<UserMetadata>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::fmt::Debug for UserMetadata {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "UserMetadata",
                "preferences",
                &self.preferences,
                "last_login",
                &self.last_login,
                "theme",
                &&self.theme,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UserMetadata {
        #[inline]
        fn clone(&self) -> UserMetadata {
            UserMetadata {
                preferences: ::core::clone::Clone::clone(&self.preferences),
                last_login: ::core::clone::Clone::clone(&self.last_login),
                theme: ::core::clone::Clone::clone(&self.theme),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UserMetadata {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UserMetadata {
        #[inline]
        fn eq(&self, other: &UserMetadata) -> bool {
            self.preferences == other.preferences && self.last_login == other.last_login
                && self.theme == other.theme
        }
    }
    pub struct UserConfig {
        pub notifications: bool,
        pub language: String,
        pub settings: std::collections::HashMap<String, String>,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for UserConfig {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "UserConfig",
                    false as usize + 1 + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "notifications",
                    &self.notifications,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "language",
                    &self.language,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "settings",
                    &self.settings,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for UserConfig {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            2u64 => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "notifications" => _serde::__private::Ok(__Field::__field0),
                            "language" => _serde::__private::Ok(__Field::__field1),
                            "settings" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"notifications" => _serde::__private::Ok(__Field::__field0),
                            b"language" => _serde::__private::Ok(__Field::__field1),
                            b"settings" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<UserConfig>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = UserConfig;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct UserConfig",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            bool,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct UserConfig with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            String,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct UserConfig with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field2 = match _serde::de::SeqAccess::next_element::<
                            std::collections::HashMap<String, String>,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        2usize,
                                        &"struct UserConfig with 3 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(UserConfig {
                            notifications: __field0,
                            language: __field1,
                            settings: __field2,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<bool> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<String> = _serde::__private::None;
                        let mut __field2: _serde::__private::Option<
                            std::collections::HashMap<String, String>,
                        > = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "notifications",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<bool>(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "language",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                __Field::__field2 => {
                                    if _serde::__private::Option::is_some(&__field2) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "settings",
                                            ),
                                        );
                                    }
                                    __field2 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            std::collections::HashMap<String, String>,
                                        >(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("notifications")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("language")?
                            }
                        };
                        let __field2 = match __field2 {
                            _serde::__private::Some(__field2) => __field2,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("settings")?
                            }
                        };
                        _serde::__private::Ok(UserConfig {
                            notifications: __field0,
                            language: __field1,
                            settings: __field2,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &[
                    "notifications",
                    "language",
                    "settings",
                ];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "UserConfig",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<UserConfig>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::fmt::Debug for UserConfig {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "UserConfig",
                "notifications",
                &self.notifications,
                "language",
                &self.language,
                "settings",
                &&self.settings,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UserConfig {
        #[inline]
        fn clone(&self) -> UserConfig {
            UserConfig {
                notifications: ::core::clone::Clone::clone(&self.notifications),
                language: ::core::clone::Clone::clone(&self.language),
                settings: ::core::clone::Clone::clone(&self.settings),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UserConfig {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UserConfig {
        #[inline]
        fn eq(&self, other: &UserConfig) -> bool {
            self.notifications == other.notifications && self.language == other.language
                && self.settings == other.settings
        }
    }
    pub fn setup_db() -> Connection {
        let conn = Connection::open_in_memory()
            .expect("Failed to create in-memory database");
        create_tables(&conn);
        conn
    }
    fn create_tables(conn: &Connection) {
        conn.execute(Simple::SQL.to_sql().sql().as_str(), [])
            .expect("Failed to create simple table");
        conn.execute(Complex::SQL.to_sql().sql().as_str(), [])
            .expect("Failed to create complex table");
        conn.execute(Post::SQL.to_sql().sql().as_str(), [])
            .expect("Failed to create posts table");
        conn.execute(Category::SQL.to_sql().sql().as_str(), [])
            .expect("Failed to create categories table");
        conn.execute(PostCategory::SQL.to_sql().sql().as_str(), [])
            .expect("Failed to create post_categories table");
    }
    pub enum Role {
        #[default]
        User,
        Admin,
    }
    impl From<Role> for i64 {
        fn from(value: Role) -> Self {
            match value {
                Role::User => 0i64,
                Role::Admin => 1i64,
            }
        }
    }
    impl From<&Role> for i64 {
        fn from(value: &Role) -> Self {
            match value {
                &Role::User => 0i64,
                &Role::Admin => 1i64,
            }
        }
    }
    impl TryFrom<i64> for Role {
        type Error = ::drizzle_rs::error::DrizzleError;
        fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
            Ok(
                match value {
                    i if i == 0i64 => Role::User,
                    i if i == 1i64 => Role::Admin,
                    _ => {
                        return Err(
                            ::drizzle_rs::error::DrizzleError::Mapping(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(format_args!("{0}", value))
                                }),
                            ),
                        );
                    }
                },
            )
        }
    }
    impl std::fmt::Display for Role {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Role::User => f.write_fmt(format_args!("User")),
                Role::Admin => f.write_fmt(format_args!("Admin")),
            }
        }
    }
    impl From<Role> for &str {
        fn from(value: Role) -> Self {
            match value {
                Role::User => "User",
                Role::Admin => "Admin",
            }
        }
    }
    impl From<&Role> for &str {
        fn from(value: &Role) -> Self {
            match value {
                &Role::User => "User",
                &Role::Admin => "Admin",
            }
        }
    }
    impl AsRef<str> for Role {
        fn as_ref(&self) -> &str {
            match self {
                Role::User => "User",
                Role::Admin => "Admin",
            }
        }
    }
    impl TryFrom<&str> for Role {
        type Error = ::drizzle_rs::error::DrizzleError;
        fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
            Ok(
                match value {
                    "User" => Role::User,
                    "Admin" => Role::Admin,
                    _ => {
                        return Err(
                            ::drizzle_rs::error::DrizzleError::Mapping(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(format_args!("{0}", value))
                                }),
                            ),
                        );
                    }
                },
            )
        }
    }
    impl std::str::FromStr for Role {
        type Err = ::drizzle_rs::error::DrizzleError;
        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(
                match s {
                    "User" => Role::User,
                    "Admin" => Role::Admin,
                    _ => {
                        return Err(
                            ::drizzle_rs::error::DrizzleError::Mapping(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(format_args!("{0}", s))
                                }),
                            ),
                        );
                    }
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for Role {
        #[inline]
        fn default() -> Role {
            Self::User
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Role {
        #[inline]
        fn clone(&self) -> Role {
            match self {
                Role::User => Role::User,
                Role::Admin => Role::Admin,
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Role {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Role {
        #[inline]
        fn eq(&self, other: &Role) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Role {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Role::User => "User",
                    Role::Admin => "Admin",
                },
            )
        }
    }
    pub struct Simple {
        pub id: SimpleId,
        pub name: SimpleName,
    }
    #[automatically_derived]
    impl ::core::default::Default for Simple {
        #[inline]
        fn default() -> Simple {
            Simple {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Simple {
        #[inline]
        fn clone(&self) -> Simple {
            let _: ::core::clone::AssertParamIsClone<SimpleId>;
            let _: ::core::clone::AssertParamIsClone<SimpleName>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Simple {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Simple {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Simple",
                "id",
                &self.id,
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Simple {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Simple {
        #[inline]
        fn eq(&self, other: &Simple) -> bool {
            self.id == other.id && self.name == other.name
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Simple {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<SimpleId>;
            let _: ::core::cmp::AssertParamIsEq<SimpleName>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Simple {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.id, state);
            ::core::hash::Hash::hash(&self.name, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Simple {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Simple,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.id, &other.id) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    ::core::cmp::PartialOrd::partial_cmp(&self.name, &other.name)
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Simple {
        #[inline]
        fn cmp(&self, other: &Simple) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.id, &other.id) {
                ::core::cmp::Ordering::Equal => {
                    ::core::cmp::Ord::cmp(&self.name, &other.name)
                }
                cmp => cmp,
            }
        }
    }
    #[allow(non_upper_case_globals)]
    impl Simple {
        const fn new() -> Self {
            Self {
                id: SimpleId::new(),
                name: SimpleName::new(),
            }
        }
        pub const id: SimpleId = SimpleId;
        pub const name: SimpleName = SimpleName;
    }
    #[allow(non_camel_case_types)]
    pub struct SimpleId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for SimpleId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "SimpleId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for SimpleId {
        #[inline]
        fn clone(&self) -> SimpleId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for SimpleId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for SimpleId {
        #[inline]
        fn default() -> SimpleId {
            SimpleId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for SimpleId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &SimpleId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for SimpleId {
        #[inline]
        fn cmp(&self, other: &SimpleId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for SimpleId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for SimpleId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for SimpleId {
        #[inline]
        fn eq(&self, other: &SimpleId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for SimpleId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl SimpleId {
        const fn new() -> SimpleId {
            SimpleId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleId {
        const NAME: &'a str = "id";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "id INTEGER PRIMARY KEY NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for SimpleId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Simple = Simple::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for SimpleId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleId {
        type Table = Simple;
        type Type = i32;
        const PRIMARY_KEY: bool = true;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for SimpleId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: SimpleId = SimpleId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for SimpleId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("id"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct SimpleName;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for SimpleName {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "SimpleName")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for SimpleName {
        #[inline]
        fn clone(&self) -> SimpleName {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for SimpleName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for SimpleName {
        #[inline]
        fn default() -> SimpleName {
            SimpleName {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for SimpleName {
        #[inline]
        fn partial_cmp(
            &self,
            other: &SimpleName,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for SimpleName {
        #[inline]
        fn cmp(&self, other: &SimpleName) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for SimpleName {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for SimpleName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for SimpleName {
        #[inline]
        fn eq(&self, other: &SimpleName) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for SimpleName {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl SimpleName {
        const fn new() -> SimpleName {
            SimpleName {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleName {
        const NAME: &'a str = "name";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "name TEXT NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for SimpleName {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Simple = Simple::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for SimpleName {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleName {
        type Table = Simple;
        type Type = String;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for SimpleName {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SimpleName {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: SimpleName = SimpleName::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for SimpleName {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("name"))
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<
        'a,
        ::drizzle_rs::core::SQLSchemaType,
        ::drizzle_rs::sqlite::SQLiteValue<'a>,
    > for Simple {
        const NAME: &'a str = "simple";
        const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "CREATE TABLE \"simple\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL);",
        );
    }
    impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Simple {
        type Select = SelectSimple;
        type Insert = InsertSimple;
        type Update = UpdateSimple;
    }
    impl ::drizzle_rs::core::SQLTableInfo for Simple {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
            Self::TYPE
        }
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            #[allow(non_upper_case_globals)]
            static SimpleId: SimpleId = SimpleId::new();
            #[allow(non_upper_case_globals)]
            static SimpleName: SimpleName = SimpleName::new();
            Box::new([SimpleId.as_column(), SimpleName.as_column()])
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Simple {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: Simple = Simple::new();
            INSTANCE.as_table().to_sql()
        }
    }
    pub struct SelectSimple {
        pub id: i32,
        pub name: String,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectSimple {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "SelectSimple",
                "id",
                &self.id,
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SelectSimple {
        #[inline]
        fn clone(&self) -> SelectSimple {
            SelectSimple {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for SelectSimple {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for SelectSimple {
        #[inline]
        fn eq(&self, other: &SelectSimple) -> bool {
            self.id == other.id && self.name == other.name
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for SelectSimple {
        #[inline]
        fn default() -> SelectSimple {
            SelectSimple {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
            }
        }
    }
    pub struct PartialSelectSimple {
        pub id: Option<i32>,
        pub name: Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PartialSelectSimple {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PartialSelectSimple",
                "id",
                &self.id,
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PartialSelectSimple {
        #[inline]
        fn clone(&self) -> PartialSelectSimple {
            PartialSelectSimple {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PartialSelectSimple {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PartialSelectSimple {
        #[inline]
        fn eq(&self, other: &PartialSelectSimple) -> bool {
            self.id == other.id && self.name == other.name
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PartialSelectSimple {
        #[inline]
        fn default() -> PartialSelectSimple {
            PartialSelectSimple {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
            }
        }
    }
    impl PartialSelectSimple {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectSimple {
        type Partial = PartialSelectSimple;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectSimple {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectSimple {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct InsertSimple {
        pub id: ::drizzle_rs::sqlite::InsertValue<i32>,
        pub name: ::drizzle_rs::sqlite::InsertValue<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for InsertSimple {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "InsertSimple",
                "id",
                &self.id,
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InsertSimple {
        #[inline]
        fn clone(&self) -> InsertSimple {
            InsertSimple {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InsertSimple {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InsertSimple {
        #[inline]
        fn eq(&self, other: &InsertSimple) -> bool {
            self.id == other.id && self.name == other.name
        }
    }
    impl Default for InsertSimple {
        fn default() -> Self {
            Self {
                id: ::drizzle_rs::sqlite::InsertValue::Omit,
                name: ::drizzle_rs::sqlite::InsertValue::Omit,
            }
        }
    }
    impl InsertSimple {
        pub fn new(name: impl Into<::std::string::String>) -> Self {
            Self {
                name: ::drizzle_rs::sqlite::InsertValue::Value(name.into()),
                ..Self::default()
            }
        }
        pub fn with_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.id = value.into();
            self
        }
        pub fn with_name<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.name = value.into();
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertSimple {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertSimple {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static TABLE: Simple = Simple::new();
            let all_columns = TABLE.columns();
            let mut result_columns = Vec::new();
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.id {} else {
                result_columns.push(all_columns[0usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.name {} else {
                result_columns.push(all_columns[1usize]);
            }
            result_columns.into_boxed_slice()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    pub struct UpdateSimple {
        pub id: ::std::option::Option<i32>,
        pub name: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdateSimple {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "UpdateSimple",
                "id",
                &self.id,
                "name",
                &&self.name,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UpdateSimple {
        #[inline]
        fn clone(&self) -> UpdateSimple {
            UpdateSimple {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdateSimple {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UpdateSimple {
        #[inline]
        fn eq(&self, other: &UpdateSimple) -> bool {
            self.id == other.id && self.name == other.name
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UpdateSimple {
        #[inline]
        fn default() -> UpdateSimple {
            UpdateSimple {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
            }
        }
    }
    impl UpdateSimple {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateSimple {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut assignments = Vec::new();
            if let Some(val) = &self.id {
                assignments
                    .push((
                        "id",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.name {
                assignments
                    .push((
                        "name",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            ::drizzle_rs::core::SQL::assignments(assignments)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectSimple {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Simple = Simple::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateSimple {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Simple = Simple::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            if let Some(val) = &self.id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.name {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectSimple {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Simple = Simple::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectSimple {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectSimple {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdateSimple {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
            })
        }
    }
    pub struct Complex {
        pub id: ComplexId,
        pub name: ComplexName,
        pub email: ComplexEmail,
        pub age: ComplexAge,
        pub score: ComplexScore,
        pub active: ComplexActive,
        pub role: ComplexRole,
        pub description: ComplexDescription,
        pub metadata: ComplexMetadata,
        pub config: ComplexConfig,
        pub data_blob: ComplexDataBlob,
        pub created_at: ComplexCreatedAt,
    }
    #[automatically_derived]
    impl ::core::default::Default for Complex {
        #[inline]
        fn default() -> Complex {
            Complex {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                email: ::core::default::Default::default(),
                age: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
                active: ::core::default::Default::default(),
                role: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
                metadata: ::core::default::Default::default(),
                config: ::core::default::Default::default(),
                data_blob: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Complex {
        #[inline]
        fn clone(&self) -> Complex {
            let _: ::core::clone::AssertParamIsClone<ComplexId>;
            let _: ::core::clone::AssertParamIsClone<ComplexName>;
            let _: ::core::clone::AssertParamIsClone<ComplexEmail>;
            let _: ::core::clone::AssertParamIsClone<ComplexAge>;
            let _: ::core::clone::AssertParamIsClone<ComplexScore>;
            let _: ::core::clone::AssertParamIsClone<ComplexActive>;
            let _: ::core::clone::AssertParamIsClone<ComplexRole>;
            let _: ::core::clone::AssertParamIsClone<ComplexDescription>;
            let _: ::core::clone::AssertParamIsClone<ComplexMetadata>;
            let _: ::core::clone::AssertParamIsClone<ComplexConfig>;
            let _: ::core::clone::AssertParamIsClone<ComplexDataBlob>;
            let _: ::core::clone::AssertParamIsClone<ComplexCreatedAt>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Complex {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Complex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "name",
                "email",
                "age",
                "score",
                "active",
                "role",
                "description",
                "metadata",
                "config",
                "data_blob",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.name,
                &self.email,
                &self.age,
                &self.score,
                &self.active,
                &self.role,
                &self.description,
                &self.metadata,
                &self.config,
                &self.data_blob,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "Complex",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Complex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Complex {
        #[inline]
        fn eq(&self, other: &Complex) -> bool {
            self.id == other.id && self.name == other.name && self.email == other.email
                && self.age == other.age && self.score == other.score
                && self.active == other.active && self.role == other.role
                && self.description == other.description
                && self.metadata == other.metadata && self.config == other.config
                && self.data_blob == other.data_blob
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Complex {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<ComplexId>;
            let _: ::core::cmp::AssertParamIsEq<ComplexName>;
            let _: ::core::cmp::AssertParamIsEq<ComplexEmail>;
            let _: ::core::cmp::AssertParamIsEq<ComplexAge>;
            let _: ::core::cmp::AssertParamIsEq<ComplexScore>;
            let _: ::core::cmp::AssertParamIsEq<ComplexActive>;
            let _: ::core::cmp::AssertParamIsEq<ComplexRole>;
            let _: ::core::cmp::AssertParamIsEq<ComplexDescription>;
            let _: ::core::cmp::AssertParamIsEq<ComplexMetadata>;
            let _: ::core::cmp::AssertParamIsEq<ComplexConfig>;
            let _: ::core::cmp::AssertParamIsEq<ComplexDataBlob>;
            let _: ::core::cmp::AssertParamIsEq<ComplexCreatedAt>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Complex {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.id, state);
            ::core::hash::Hash::hash(&self.name, state);
            ::core::hash::Hash::hash(&self.email, state);
            ::core::hash::Hash::hash(&self.age, state);
            ::core::hash::Hash::hash(&self.score, state);
            ::core::hash::Hash::hash(&self.active, state);
            ::core::hash::Hash::hash(&self.role, state);
            ::core::hash::Hash::hash(&self.description, state);
            ::core::hash::Hash::hash(&self.metadata, state);
            ::core::hash::Hash::hash(&self.config, state);
            ::core::hash::Hash::hash(&self.data_blob, state);
            ::core::hash::Hash::hash(&self.created_at, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Complex {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Complex,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.id, &other.id) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(&self.name, &other.name) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            match ::core::cmp::PartialOrd::partial_cmp(
                                &self.email,
                                &other.email,
                            ) {
                                ::core::option::Option::Some(
                                    ::core::cmp::Ordering::Equal,
                                ) => {
                                    match ::core::cmp::PartialOrd::partial_cmp(
                                        &self.age,
                                        &other.age,
                                    ) {
                                        ::core::option::Option::Some(
                                            ::core::cmp::Ordering::Equal,
                                        ) => {
                                            match ::core::cmp::PartialOrd::partial_cmp(
                                                &self.score,
                                                &other.score,
                                            ) {
                                                ::core::option::Option::Some(
                                                    ::core::cmp::Ordering::Equal,
                                                ) => {
                                                    match ::core::cmp::PartialOrd::partial_cmp(
                                                        &self.active,
                                                        &other.active,
                                                    ) {
                                                        ::core::option::Option::Some(
                                                            ::core::cmp::Ordering::Equal,
                                                        ) => {
                                                            match ::core::cmp::PartialOrd::partial_cmp(
                                                                &self.role,
                                                                &other.role,
                                                            ) {
                                                                ::core::option::Option::Some(
                                                                    ::core::cmp::Ordering::Equal,
                                                                ) => {
                                                                    match ::core::cmp::PartialOrd::partial_cmp(
                                                                        &self.description,
                                                                        &other.description,
                                                                    ) {
                                                                        ::core::option::Option::Some(
                                                                            ::core::cmp::Ordering::Equal,
                                                                        ) => {
                                                                            match ::core::cmp::PartialOrd::partial_cmp(
                                                                                &self.metadata,
                                                                                &other.metadata,
                                                                            ) {
                                                                                ::core::option::Option::Some(
                                                                                    ::core::cmp::Ordering::Equal,
                                                                                ) => {
                                                                                    match ::core::cmp::PartialOrd::partial_cmp(
                                                                                        &self.config,
                                                                                        &other.config,
                                                                                    ) {
                                                                                        ::core::option::Option::Some(
                                                                                            ::core::cmp::Ordering::Equal,
                                                                                        ) => {
                                                                                            match ::core::cmp::PartialOrd::partial_cmp(
                                                                                                &self.data_blob,
                                                                                                &other.data_blob,
                                                                                            ) {
                                                                                                ::core::option::Option::Some(
                                                                                                    ::core::cmp::Ordering::Equal,
                                                                                                ) => {
                                                                                                    ::core::cmp::PartialOrd::partial_cmp(
                                                                                                        &self.created_at,
                                                                                                        &other.created_at,
                                                                                                    )
                                                                                                }
                                                                                                cmp => cmp,
                                                                                            }
                                                                                        }
                                                                                        cmp => cmp,
                                                                                    }
                                                                                }
                                                                                cmp => cmp,
                                                                            }
                                                                        }
                                                                        cmp => cmp,
                                                                    }
                                                                }
                                                                cmp => cmp,
                                                            }
                                                        }
                                                        cmp => cmp,
                                                    }
                                                }
                                                cmp => cmp,
                                            }
                                        }
                                        cmp => cmp,
                                    }
                                }
                                cmp => cmp,
                            }
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Complex {
        #[inline]
        fn cmp(&self, other: &Complex) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.id, &other.id) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.name, &other.name) {
                        ::core::cmp::Ordering::Equal => {
                            match ::core::cmp::Ord::cmp(&self.email, &other.email) {
                                ::core::cmp::Ordering::Equal => {
                                    match ::core::cmp::Ord::cmp(&self.age, &other.age) {
                                        ::core::cmp::Ordering::Equal => {
                                            match ::core::cmp::Ord::cmp(&self.score, &other.score) {
                                                ::core::cmp::Ordering::Equal => {
                                                    match ::core::cmp::Ord::cmp(&self.active, &other.active) {
                                                        ::core::cmp::Ordering::Equal => {
                                                            match ::core::cmp::Ord::cmp(&self.role, &other.role) {
                                                                ::core::cmp::Ordering::Equal => {
                                                                    match ::core::cmp::Ord::cmp(
                                                                        &self.description,
                                                                        &other.description,
                                                                    ) {
                                                                        ::core::cmp::Ordering::Equal => {
                                                                            match ::core::cmp::Ord::cmp(
                                                                                &self.metadata,
                                                                                &other.metadata,
                                                                            ) {
                                                                                ::core::cmp::Ordering::Equal => {
                                                                                    match ::core::cmp::Ord::cmp(&self.config, &other.config) {
                                                                                        ::core::cmp::Ordering::Equal => {
                                                                                            match ::core::cmp::Ord::cmp(
                                                                                                &self.data_blob,
                                                                                                &other.data_blob,
                                                                                            ) {
                                                                                                ::core::cmp::Ordering::Equal => {
                                                                                                    ::core::cmp::Ord::cmp(&self.created_at, &other.created_at)
                                                                                                }
                                                                                                cmp => cmp,
                                                                                            }
                                                                                        }
                                                                                        cmp => cmp,
                                                                                    }
                                                                                }
                                                                                cmp => cmp,
                                                                            }
                                                                        }
                                                                        cmp => cmp,
                                                                    }
                                                                }
                                                                cmp => cmp,
                                                            }
                                                        }
                                                        cmp => cmp,
                                                    }
                                                }
                                                cmp => cmp,
                                            }
                                        }
                                        cmp => cmp,
                                    }
                                }
                                cmp => cmp,
                            }
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[allow(non_upper_case_globals)]
    impl Complex {
        const fn new() -> Self {
            Self {
                id: ComplexId::new(),
                name: ComplexName::new(),
                email: ComplexEmail::new(),
                age: ComplexAge::new(),
                score: ComplexScore::new(),
                active: ComplexActive::new(),
                role: ComplexRole::new(),
                description: ComplexDescription::new(),
                metadata: ComplexMetadata::new(),
                config: ComplexConfig::new(),
                data_blob: ComplexDataBlob::new(),
                created_at: ComplexCreatedAt::new(),
            }
        }
        pub const id: ComplexId = ComplexId;
        pub const name: ComplexName = ComplexName;
        pub const email: ComplexEmail = ComplexEmail;
        pub const age: ComplexAge = ComplexAge;
        pub const score: ComplexScore = ComplexScore;
        pub const active: ComplexActive = ComplexActive;
        pub const role: ComplexRole = ComplexRole;
        pub const description: ComplexDescription = ComplexDescription;
        pub const metadata: ComplexMetadata = ComplexMetadata;
        pub const config: ComplexConfig = ComplexConfig;
        pub const data_blob: ComplexDataBlob = ComplexDataBlob;
        pub const created_at: ComplexCreatedAt = ComplexCreatedAt;
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexId {
        #[inline]
        fn clone(&self) -> ComplexId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexId {
        #[inline]
        fn default() -> ComplexId {
            ComplexId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexId {
        #[inline]
        fn cmp(&self, other: &ComplexId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexId {
        #[inline]
        fn eq(&self, other: &ComplexId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexId {
        const fn new() -> ComplexId {
            ComplexId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexId {
        const NAME: &'a str = "id";
        const TYPE: &'a str = "BLOB";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "id BLOB PRIMARY KEY NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            true
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexId {
        type Table = Complex;
        type Type = Uuid;
        const PRIMARY_KEY: bool = true;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            Some(Uuid::new_v4)
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexId = ComplexId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for ComplexId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("id"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexName;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexName {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexName")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexName {
        #[inline]
        fn clone(&self) -> ComplexName {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexName {
        #[inline]
        fn default() -> ComplexName {
            ComplexName {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexName {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexName,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexName {
        #[inline]
        fn cmp(&self, other: &ComplexName) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexName {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexName {
        #[inline]
        fn eq(&self, other: &ComplexName) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexName {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexName {
        const fn new() -> ComplexName {
            ComplexName {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexName {
        const NAME: &'a str = "name";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "name TEXT NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexName {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexName {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexName {
        type Table = Complex;
        type Type = String;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexName {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexName {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexName = ComplexName::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexName {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("name"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexEmail;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexEmail {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexEmail")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexEmail {
        #[inline]
        fn clone(&self) -> ComplexEmail {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexEmail {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexEmail {
        #[inline]
        fn default() -> ComplexEmail {
            ComplexEmail {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexEmail {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexEmail,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexEmail {
        #[inline]
        fn cmp(&self, other: &ComplexEmail) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexEmail {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexEmail {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexEmail {
        #[inline]
        fn eq(&self, other: &ComplexEmail) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexEmail {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexEmail {
        const fn new() -> ComplexEmail {
            ComplexEmail {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexEmail {
        const NAME: &'a str = "email";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "email TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexEmail {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexEmail {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexEmail {
        type Table = Complex;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexEmail {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexEmail {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexEmail = ComplexEmail::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexEmail {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("email"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexAge;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexAge {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexAge")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexAge {
        #[inline]
        fn clone(&self) -> ComplexAge {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexAge {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexAge {
        #[inline]
        fn default() -> ComplexAge {
            ComplexAge {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexAge {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexAge,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexAge {
        #[inline]
        fn cmp(&self, other: &ComplexAge) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexAge {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexAge {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexAge {
        #[inline]
        fn eq(&self, other: &ComplexAge) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexAge {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexAge {
        const fn new() -> ComplexAge {
            ComplexAge {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexAge {
        const NAME: &'a str = "age";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "age INTEGER",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexAge {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexAge {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexAge {
        type Table = Complex;
        type Type = Option<i32>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexAge {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexAge {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexAge = ComplexAge::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for ComplexAge {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("age"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexScore;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexScore {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexScore")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexScore {
        #[inline]
        fn clone(&self) -> ComplexScore {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexScore {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexScore {
        #[inline]
        fn default() -> ComplexScore {
            ComplexScore {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexScore {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexScore,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexScore {
        #[inline]
        fn cmp(&self, other: &ComplexScore) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexScore {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexScore {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexScore {
        #[inline]
        fn eq(&self, other: &ComplexScore) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexScore {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexScore {
        const fn new() -> ComplexScore {
            ComplexScore {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexScore {
        const NAME: &'a str = "score";
        const TYPE: &'a str = "REAL";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "score REAL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexScore {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexScore {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexScore {
        type Table = Complex;
        type Type = Option<f64>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexScore {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexScore {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexScore = ComplexScore::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexScore {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("score"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexActive;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexActive {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexActive")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexActive {
        #[inline]
        fn clone(&self) -> ComplexActive {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexActive {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexActive {
        #[inline]
        fn default() -> ComplexActive {
            ComplexActive {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexActive {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexActive,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexActive {
        #[inline]
        fn cmp(&self, other: &ComplexActive) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexActive {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexActive {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexActive {
        #[inline]
        fn eq(&self, other: &ComplexActive) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexActive {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexActive {
        const fn new() -> ComplexActive {
            ComplexActive {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexActive {
        const NAME: &'a str = "active";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "active INTEGER NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexActive {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexActive {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexActive {
        type Table = Complex;
        type Type = bool;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexActive {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexActive {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexActive = ComplexActive::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexActive {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("active"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexRole;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexRole {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexRole")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexRole {
        #[inline]
        fn clone(&self) -> ComplexRole {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexRole {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexRole {
        #[inline]
        fn default() -> ComplexRole {
            ComplexRole {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexRole {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexRole,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexRole {
        #[inline]
        fn cmp(&self, other: &ComplexRole) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexRole {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexRole {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexRole {
        #[inline]
        fn eq(&self, other: &ComplexRole) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexRole {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexRole {
        const fn new() -> ComplexRole {
            ComplexRole {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexRole {
        const NAME: &'a str = "role";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "role TEXT NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexRole {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexRole {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexRole {
        type Table = Complex;
        type Type = Role;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexRole {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexRole {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexRole = ComplexRole::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexRole {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("role"))
        }
    }
    impl<'a> ::std::convert::From<Role> for ::drizzle_rs::sqlite::SQLiteValue<'a> {
        fn from(value: Role) -> Self {
            let text: &str = value.into();
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
        }
    }
    impl<'a> ::std::convert::From<&'a Role> for ::drizzle_rs::sqlite::SQLiteValue<'a> {
        fn from(value: &'a Role) -> Self {
            let text: &str = value.into();
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Role {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let value = self;
            let text: &str = value.into();
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
                .into()
        }
    }
    impl ::rusqlite::types::FromSql for Role {
        fn column_result(
            value: ::rusqlite::types::ValueRef<'_>,
        ) -> ::rusqlite::types::FromSqlResult<Self> {
            match value {
                ::rusqlite::types::ValueRef::Text(s) => {
                    let s_str = ::std::str::from_utf8(s)
                        .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)?;
                    Self::try_from(s_str)
                        .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                }
                _ => Err(::rusqlite::types::FromSqlError::InvalidType),
            }
        }
    }
    impl ::rusqlite::types::ToSql for Role {
        fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
            let val: &str = self.into();
            Ok(
                ::rusqlite::types::ToSqlOutput::Borrowed(
                    ::rusqlite::types::ValueRef::Text(val.as_bytes()),
                ),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexDescription;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexDescription {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexDescription")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexDescription {
        #[inline]
        fn clone(&self) -> ComplexDescription {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexDescription {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexDescription {
        #[inline]
        fn default() -> ComplexDescription {
            ComplexDescription {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexDescription {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexDescription,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexDescription {
        #[inline]
        fn cmp(&self, other: &ComplexDescription) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexDescription {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexDescription {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexDescription {
        #[inline]
        fn eq(&self, other: &ComplexDescription) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexDescription {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexDescription {
        const fn new() -> ComplexDescription {
            ComplexDescription {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDescription {
        const NAME: &'a str = "description";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "description TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexDescription {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexDescription {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDescription {
        type Table = Complex;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexDescription {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDescription {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexDescription = ComplexDescription::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDescription {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("description"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexMetadata;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexMetadata {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexMetadata")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexMetadata {
        #[inline]
        fn clone(&self) -> ComplexMetadata {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexMetadata {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexMetadata {
        #[inline]
        fn default() -> ComplexMetadata {
            ComplexMetadata {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexMetadata {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexMetadata,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexMetadata {
        #[inline]
        fn cmp(&self, other: &ComplexMetadata) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexMetadata {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexMetadata {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexMetadata {
        #[inline]
        fn eq(&self, other: &ComplexMetadata) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexMetadata {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexMetadata {
        const fn new() -> ComplexMetadata {
            ComplexMetadata {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexMetadata {
        const NAME: &'a str = "metadata";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "metadata TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexMetadata {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexMetadata {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexMetadata {
        type Table = Complex;
        type Type = Option<UserMetadata>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexMetadata {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexMetadata {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexMetadata = ComplexMetadata::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexMetadata {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("metadata"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexConfig;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexConfig {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexConfig")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexConfig {
        #[inline]
        fn clone(&self) -> ComplexConfig {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexConfig {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexConfig {
        #[inline]
        fn default() -> ComplexConfig {
            ComplexConfig {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexConfig {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexConfig,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexConfig {
        #[inline]
        fn cmp(&self, other: &ComplexConfig) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexConfig {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexConfig {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexConfig {
        #[inline]
        fn eq(&self, other: &ComplexConfig) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexConfig {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexConfig {
        const fn new() -> ComplexConfig {
            ComplexConfig {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexConfig {
        const NAME: &'a str = "config";
        const TYPE: &'a str = "BLOB";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "config BLOB",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexConfig {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexConfig {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexConfig {
        type Table = Complex;
        type Type = Option<UserConfig>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexConfig {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexConfig {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexConfig = ComplexConfig::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexConfig {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("config"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexDataBlob;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexDataBlob {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexDataBlob")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexDataBlob {
        #[inline]
        fn clone(&self) -> ComplexDataBlob {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexDataBlob {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexDataBlob {
        #[inline]
        fn default() -> ComplexDataBlob {
            ComplexDataBlob {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexDataBlob {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexDataBlob,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexDataBlob {
        #[inline]
        fn cmp(&self, other: &ComplexDataBlob) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexDataBlob {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexDataBlob {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexDataBlob {
        #[inline]
        fn eq(&self, other: &ComplexDataBlob) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexDataBlob {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexDataBlob {
        const fn new() -> ComplexDataBlob {
            ComplexDataBlob {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDataBlob {
        const NAME: &'a str = "data_blob";
        const TYPE: &'a str = "BLOB";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "data_blob BLOB",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexDataBlob {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexDataBlob {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDataBlob {
        type Table = Complex;
        type Type = Option<Vec<u8>>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexDataBlob {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDataBlob {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexDataBlob = ComplexDataBlob::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexDataBlob {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("data_blob"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct ComplexCreatedAt;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for ComplexCreatedAt {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ComplexCreatedAt")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for ComplexCreatedAt {
        #[inline]
        fn clone(&self) -> ComplexCreatedAt {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for ComplexCreatedAt {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for ComplexCreatedAt {
        #[inline]
        fn default() -> ComplexCreatedAt {
            ComplexCreatedAt {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for ComplexCreatedAt {
        #[inline]
        fn partial_cmp(
            &self,
            other: &ComplexCreatedAt,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for ComplexCreatedAt {
        #[inline]
        fn cmp(&self, other: &ComplexCreatedAt) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for ComplexCreatedAt {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for ComplexCreatedAt {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for ComplexCreatedAt {
        #[inline]
        fn eq(&self, other: &ComplexCreatedAt) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for ComplexCreatedAt {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl ComplexCreatedAt {
        const fn new() -> ComplexCreatedAt {
            ComplexCreatedAt {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexCreatedAt {
        const NAME: &'a str = "created_at";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "created_at TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for ComplexCreatedAt {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Complex = Complex::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for ComplexCreatedAt {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexCreatedAt {
        type Table = Complex;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for ComplexCreatedAt {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexCreatedAt {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: ComplexCreatedAt = ComplexCreatedAt::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for ComplexCreatedAt {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("created_at"),
            )
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<
        'a,
        ::drizzle_rs::core::SQLSchemaType,
        ::drizzle_rs::sqlite::SQLiteValue<'a>,
    > for Complex {
        const NAME: &'a str = "complex";
        const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "CREATE TABLE \"complex\" (id BLOB PRIMARY KEY NOT NULL, name TEXT NOT NULL, email TEXT, age INTEGER, score REAL, active INTEGER NOT NULL, role TEXT NOT NULL, description TEXT, metadata TEXT, config BLOB, data_blob BLOB, created_at TEXT);",
        );
    }
    impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Complex {
        type Select = SelectComplex;
        type Insert = InsertComplex;
        type Update = UpdateComplex;
    }
    impl ::drizzle_rs::core::SQLTableInfo for Complex {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
            Self::TYPE
        }
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            #[allow(non_upper_case_globals)]
            static ComplexId: ComplexId = ComplexId::new();
            #[allow(non_upper_case_globals)]
            static ComplexName: ComplexName = ComplexName::new();
            #[allow(non_upper_case_globals)]
            static ComplexEmail: ComplexEmail = ComplexEmail::new();
            #[allow(non_upper_case_globals)]
            static ComplexAge: ComplexAge = ComplexAge::new();
            #[allow(non_upper_case_globals)]
            static ComplexScore: ComplexScore = ComplexScore::new();
            #[allow(non_upper_case_globals)]
            static ComplexActive: ComplexActive = ComplexActive::new();
            #[allow(non_upper_case_globals)]
            static ComplexRole: ComplexRole = ComplexRole::new();
            #[allow(non_upper_case_globals)]
            static ComplexDescription: ComplexDescription = ComplexDescription::new();
            #[allow(non_upper_case_globals)]
            static ComplexMetadata: ComplexMetadata = ComplexMetadata::new();
            #[allow(non_upper_case_globals)]
            static ComplexConfig: ComplexConfig = ComplexConfig::new();
            #[allow(non_upper_case_globals)]
            static ComplexDataBlob: ComplexDataBlob = ComplexDataBlob::new();
            #[allow(non_upper_case_globals)]
            static ComplexCreatedAt: ComplexCreatedAt = ComplexCreatedAt::new();
            Box::new([
                ComplexId.as_column(),
                ComplexName.as_column(),
                ComplexEmail.as_column(),
                ComplexAge.as_column(),
                ComplexScore.as_column(),
                ComplexActive.as_column(),
                ComplexRole.as_column(),
                ComplexDescription.as_column(),
                ComplexMetadata.as_column(),
                ComplexConfig.as_column(),
                ComplexDataBlob.as_column(),
                ComplexCreatedAt.as_column(),
            ])
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Complex {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: Complex = Complex::new();
            INSTANCE.as_table().to_sql()
        }
    }
    pub struct SelectComplex {
        pub id: Uuid,
        pub name: String,
        pub email: ::std::option::Option<String>,
        pub age: ::std::option::Option<i32>,
        pub score: ::std::option::Option<f64>,
        pub active: bool,
        pub role: Role,
        pub description: ::std::option::Option<String>,
        pub metadata: ::std::option::Option<UserMetadata>,
        pub config: ::std::option::Option<UserConfig>,
        pub data_blob: ::std::option::Option<Vec<u8>>,
        pub created_at: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectComplex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "name",
                "email",
                "age",
                "score",
                "active",
                "role",
                "description",
                "metadata",
                "config",
                "data_blob",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.name,
                &self.email,
                &self.age,
                &self.score,
                &self.active,
                &self.role,
                &self.description,
                &self.metadata,
                &self.config,
                &self.data_blob,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "SelectComplex",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SelectComplex {
        #[inline]
        fn clone(&self) -> SelectComplex {
            SelectComplex {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                email: ::core::clone::Clone::clone(&self.email),
                age: ::core::clone::Clone::clone(&self.age),
                score: ::core::clone::Clone::clone(&self.score),
                active: ::core::clone::Clone::clone(&self.active),
                role: ::core::clone::Clone::clone(&self.role),
                description: ::core::clone::Clone::clone(&self.description),
                metadata: ::core::clone::Clone::clone(&self.metadata),
                config: ::core::clone::Clone::clone(&self.config),
                data_blob: ::core::clone::Clone::clone(&self.data_blob),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for SelectComplex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for SelectComplex {
        #[inline]
        fn eq(&self, other: &SelectComplex) -> bool {
            self.active == other.active && self.id == other.id && self.name == other.name
                && self.email == other.email && self.age == other.age
                && self.score == other.score && self.role == other.role
                && self.description == other.description
                && self.metadata == other.metadata && self.config == other.config
                && self.data_blob == other.data_blob
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for SelectComplex {
        #[inline]
        fn default() -> SelectComplex {
            SelectComplex {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                email: ::core::default::Default::default(),
                age: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
                active: ::core::default::Default::default(),
                role: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
                metadata: ::core::default::Default::default(),
                config: ::core::default::Default::default(),
                data_blob: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    pub struct PartialSelectComplex {
        pub id: Option<Uuid>,
        pub name: Option<String>,
        pub email: Option<String>,
        pub age: Option<i32>,
        pub score: Option<f64>,
        pub active: Option<bool>,
        pub role: Option<Role>,
        pub description: Option<String>,
        pub metadata: Option<UserMetadata>,
        pub config: Option<UserConfig>,
        pub data_blob: Option<Vec<u8>>,
        pub created_at: Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PartialSelectComplex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "name",
                "email",
                "age",
                "score",
                "active",
                "role",
                "description",
                "metadata",
                "config",
                "data_blob",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.name,
                &self.email,
                &self.age,
                &self.score,
                &self.active,
                &self.role,
                &self.description,
                &self.metadata,
                &self.config,
                &self.data_blob,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "PartialSelectComplex",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PartialSelectComplex {
        #[inline]
        fn clone(&self) -> PartialSelectComplex {
            PartialSelectComplex {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                email: ::core::clone::Clone::clone(&self.email),
                age: ::core::clone::Clone::clone(&self.age),
                score: ::core::clone::Clone::clone(&self.score),
                active: ::core::clone::Clone::clone(&self.active),
                role: ::core::clone::Clone::clone(&self.role),
                description: ::core::clone::Clone::clone(&self.description),
                metadata: ::core::clone::Clone::clone(&self.metadata),
                config: ::core::clone::Clone::clone(&self.config),
                data_blob: ::core::clone::Clone::clone(&self.data_blob),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PartialSelectComplex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PartialSelectComplex {
        #[inline]
        fn eq(&self, other: &PartialSelectComplex) -> bool {
            self.id == other.id && self.name == other.name && self.email == other.email
                && self.age == other.age && self.score == other.score
                && self.active == other.active && self.role == other.role
                && self.description == other.description
                && self.metadata == other.metadata && self.config == other.config
                && self.data_blob == other.data_blob
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PartialSelectComplex {
        #[inline]
        fn default() -> PartialSelectComplex {
            PartialSelectComplex {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                email: ::core::default::Default::default(),
                age: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
                active: ::core::default::Default::default(),
                role: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
                metadata: ::core::default::Default::default(),
                config: ::core::default::Default::default(),
                data_blob: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    impl PartialSelectComplex {
        pub fn with_id<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
            let value = value.into();
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
        pub fn with_email<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.email = Some(value);
            self
        }
        pub fn with_age(mut self, value: i32) -> Self {
            self.age = Some(value);
            self
        }
        pub fn with_score(mut self, value: f64) -> Self {
            self.score = Some(value);
            self
        }
        pub fn with_active(mut self, value: bool) -> Self {
            self.active = Some(value);
            self
        }
        pub fn with_role(mut self, value: Role) -> Self {
            self.role = Some(value);
            self
        }
        pub fn with_description<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.description = Some(value);
            self
        }
        pub fn with_metadata(mut self, value: UserMetadata) -> Self {
            self.metadata = Some(value);
            self
        }
        pub fn with_config(mut self, value: UserConfig) -> Self {
            self.config = Some(value);
            self
        }
        pub fn with_data_blob<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
            let value = value.into();
            self.data_blob = Some(value);
            self
        }
        pub fn with_created_at<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.created_at = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectComplex {
        type Partial = PartialSelectComplex;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectComplex {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectComplex {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct InsertComplex {
        pub id: ::drizzle_rs::sqlite::InsertValue<Uuid>,
        pub name: ::drizzle_rs::sqlite::InsertValue<String>,
        pub email: ::drizzle_rs::sqlite::InsertValue<String>,
        pub age: ::drizzle_rs::sqlite::InsertValue<i32>,
        pub score: ::drizzle_rs::sqlite::InsertValue<f64>,
        pub active: ::drizzle_rs::sqlite::InsertValue<bool>,
        pub role: ::drizzle_rs::sqlite::InsertValue<Role>,
        pub description: ::drizzle_rs::sqlite::InsertValue<String>,
        pub metadata: ::drizzle_rs::sqlite::InsertValue<UserMetadata>,
        pub config: ::drizzle_rs::sqlite::InsertValue<UserConfig>,
        pub data_blob: ::drizzle_rs::sqlite::InsertValue<Vec<u8>>,
        pub created_at: ::drizzle_rs::sqlite::InsertValue<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for InsertComplex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "name",
                "email",
                "age",
                "score",
                "active",
                "role",
                "description",
                "metadata",
                "config",
                "data_blob",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.name,
                &self.email,
                &self.age,
                &self.score,
                &self.active,
                &self.role,
                &self.description,
                &self.metadata,
                &self.config,
                &self.data_blob,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "InsertComplex",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InsertComplex {
        #[inline]
        fn clone(&self) -> InsertComplex {
            InsertComplex {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                email: ::core::clone::Clone::clone(&self.email),
                age: ::core::clone::Clone::clone(&self.age),
                score: ::core::clone::Clone::clone(&self.score),
                active: ::core::clone::Clone::clone(&self.active),
                role: ::core::clone::Clone::clone(&self.role),
                description: ::core::clone::Clone::clone(&self.description),
                metadata: ::core::clone::Clone::clone(&self.metadata),
                config: ::core::clone::Clone::clone(&self.config),
                data_blob: ::core::clone::Clone::clone(&self.data_blob),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InsertComplex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InsertComplex {
        #[inline]
        fn eq(&self, other: &InsertComplex) -> bool {
            self.id == other.id && self.name == other.name && self.email == other.email
                && self.age == other.age && self.score == other.score
                && self.active == other.active && self.role == other.role
                && self.description == other.description
                && self.metadata == other.metadata && self.config == other.config
                && self.data_blob == other.data_blob
                && self.created_at == other.created_at
        }
    }
    impl Default for InsertComplex {
        fn default() -> Self {
            Self {
                id: ::drizzle_rs::sqlite::InsertValue::Value((Uuid::new_v4)()),
                name: ::drizzle_rs::sqlite::InsertValue::Omit,
                email: ::drizzle_rs::sqlite::InsertValue::Omit,
                age: ::drizzle_rs::sqlite::InsertValue::Omit,
                score: ::drizzle_rs::sqlite::InsertValue::Omit,
                active: ::drizzle_rs::sqlite::InsertValue::Omit,
                role: ::drizzle_rs::sqlite::InsertValue::Omit,
                description: ::drizzle_rs::sqlite::InsertValue::Omit,
                metadata: ::drizzle_rs::sqlite::InsertValue::Omit,
                config: ::drizzle_rs::sqlite::InsertValue::Omit,
                data_blob: ::drizzle_rs::sqlite::InsertValue::Omit,
                created_at: ::drizzle_rs::sqlite::InsertValue::Omit,
            }
        }
    }
    impl InsertComplex {
        pub fn new(
            name: impl Into<::std::string::String>,
            active: bool,
            role: Role,
        ) -> Self {
            Self {
                name: ::drizzle_rs::sqlite::InsertValue::Value(name.into()),
                active: ::drizzle_rs::sqlite::InsertValue::Value(active),
                role: ::drizzle_rs::sqlite::InsertValue::Value(role),
                ..Self::default()
            }
        }
        pub fn with_id<V: Into<::drizzle_rs::sqlite::InsertValue<::uuid::Uuid>>>(
            mut self,
            value: V,
        ) -> Self {
            self.id = value.into();
            self
        }
        pub fn with_name<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.name = value.into();
            self
        }
        pub fn with_email<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.email = value.into();
            self
        }
        pub fn with_age<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.age = value.into();
            self
        }
        pub fn with_score<V: Into<::drizzle_rs::sqlite::InsertValue<f64>>>(
            mut self,
            value: V,
        ) -> Self {
            self.score = value.into();
            self
        }
        pub fn with_active<V: Into<::drizzle_rs::sqlite::InsertValue<bool>>>(
            mut self,
            value: V,
        ) -> Self {
            self.active = value.into();
            self
        }
        pub fn with_role<V: Into<::drizzle_rs::sqlite::InsertValue<Role>>>(
            mut self,
            value: V,
        ) -> Self {
            self.role = value.into();
            self
        }
        pub fn with_description<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.description = value.into();
            self
        }
        pub fn with_metadata<V: Into<::drizzle_rs::sqlite::InsertValue<UserMetadata>>>(
            mut self,
            value: V,
        ) -> Self {
            self.metadata = value.into();
            self
        }
        pub fn with_config<V: Into<::drizzle_rs::sqlite::InsertValue<UserConfig>>>(
            mut self,
            value: V,
        ) -> Self {
            self.config = value.into();
            self
        }
        pub fn with_data_blob<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::vec::Vec<u8>>>,
        >(mut self, value: V) -> Self {
            self.data_blob = value.into();
            self
        }
        pub fn with_created_at<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.created_at = value.into();
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertComplex {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.email {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.age {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.score {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.active {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.role {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.description {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.metadata {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.config {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.data_blob {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.created_at {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertComplex {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static TABLE: Complex = Complex::new();
            let all_columns = TABLE.columns();
            let mut result_columns = Vec::new();
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.id {} else {
                result_columns.push(all_columns[0usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.name {} else {
                result_columns.push(all_columns[1usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.email {} else {
                result_columns.push(all_columns[2usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.age {} else {
                result_columns.push(all_columns[3usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.score {} else {
                result_columns.push(all_columns[4usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.active {} else {
                result_columns.push(all_columns[5usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.role {} else {
                result_columns.push(all_columns[6usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.description {} else {
                result_columns.push(all_columns[7usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.metadata {} else {
                result_columns.push(all_columns[8usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.config {} else {
                result_columns.push(all_columns[9usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.data_blob {} else {
                result_columns.push(all_columns[10usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.created_at {} else {
                result_columns.push(all_columns[11usize]);
            }
            result_columns.into_boxed_slice()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.email {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.age {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.score {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.active {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.role {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.description {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.metadata {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.config {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.data_blob {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.created_at {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    pub struct UpdateComplex {
        pub id: ::std::option::Option<Uuid>,
        pub name: ::std::option::Option<String>,
        pub email: ::std::option::Option<String>,
        pub age: ::std::option::Option<i32>,
        pub score: ::std::option::Option<f64>,
        pub active: ::std::option::Option<bool>,
        pub role: ::std::option::Option<Role>,
        pub description: ::std::option::Option<String>,
        pub metadata: ::std::option::Option<UserMetadata>,
        pub config: ::std::option::Option<UserConfig>,
        pub data_blob: ::std::option::Option<Vec<u8>>,
        pub created_at: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdateComplex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "name",
                "email",
                "age",
                "score",
                "active",
                "role",
                "description",
                "metadata",
                "config",
                "data_blob",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.name,
                &self.email,
                &self.age,
                &self.score,
                &self.active,
                &self.role,
                &self.description,
                &self.metadata,
                &self.config,
                &self.data_blob,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "UpdateComplex",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UpdateComplex {
        #[inline]
        fn clone(&self) -> UpdateComplex {
            UpdateComplex {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                email: ::core::clone::Clone::clone(&self.email),
                age: ::core::clone::Clone::clone(&self.age),
                score: ::core::clone::Clone::clone(&self.score),
                active: ::core::clone::Clone::clone(&self.active),
                role: ::core::clone::Clone::clone(&self.role),
                description: ::core::clone::Clone::clone(&self.description),
                metadata: ::core::clone::Clone::clone(&self.metadata),
                config: ::core::clone::Clone::clone(&self.config),
                data_blob: ::core::clone::Clone::clone(&self.data_blob),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdateComplex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UpdateComplex {
        #[inline]
        fn eq(&self, other: &UpdateComplex) -> bool {
            self.id == other.id && self.name == other.name && self.email == other.email
                && self.age == other.age && self.score == other.score
                && self.active == other.active && self.role == other.role
                && self.description == other.description
                && self.metadata == other.metadata && self.config == other.config
                && self.data_blob == other.data_blob
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UpdateComplex {
        #[inline]
        fn default() -> UpdateComplex {
            UpdateComplex {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                email: ::core::default::Default::default(),
                age: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
                active: ::core::default::Default::default(),
                role: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
                metadata: ::core::default::Default::default(),
                config: ::core::default::Default::default(),
                data_blob: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    impl UpdateComplex {
        pub fn with_id<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
            let value = value.into();
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
        pub fn with_email<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.email = Some(value);
            self
        }
        pub fn with_age(mut self, value: i32) -> Self {
            self.age = Some(value);
            self
        }
        pub fn with_score(mut self, value: f64) -> Self {
            self.score = Some(value);
            self
        }
        pub fn with_active(mut self, value: bool) -> Self {
            self.active = Some(value);
            self
        }
        pub fn with_role(mut self, value: Role) -> Self {
            self.role = Some(value);
            self
        }
        pub fn with_description<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.description = Some(value);
            self
        }
        pub fn with_metadata(mut self, value: UserMetadata) -> Self {
            self.metadata = Some(value);
            self
        }
        pub fn with_config(mut self, value: UserConfig) -> Self {
            self.config = Some(value);
            self
        }
        pub fn with_data_blob<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
            let value = value.into();
            self.data_blob = Some(value);
            self
        }
        pub fn with_created_at<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.created_at = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateComplex {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut assignments = Vec::new();
            if let Some(val) = &self.id {
                assignments
                    .push((
                        "id",
                        ::drizzle_rs::sqlite::SQLiteValue::Blob(
                            ::std::borrow::Cow::Owned(val.as_bytes().to_vec()),
                        ),
                    ));
            }
            if let Some(val) = &self.name {
                assignments
                    .push((
                        "name",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.email {
                assignments
                    .push((
                        "email",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.age {
                assignments
                    .push((
                        "age",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.score {
                assignments
                    .push((
                        "score",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.active {
                assignments
                    .push((
                        "active",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.role {
                assignments.push(("role", val.clone().into()));
            }
            if let Some(val) = &self.description {
                assignments
                    .push((
                        "description",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.metadata {
                assignments
                    .push((
                        "metadata",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.config {
                assignments
                    .push((
                        "config",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.data_blob {
                assignments
                    .push((
                        "data_blob",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.created_at {
                assignments
                    .push((
                        "created_at",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            ::drizzle_rs::core::SQL::assignments(assignments)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectComplex {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Complex = Complex::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateComplex {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Complex = Complex::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            if let Some(val) = &self.id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.name {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.email {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.age {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.score {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.active {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.role {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.description {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.metadata {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.config {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.data_blob {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.created_at {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectComplex {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Complex = Complex::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl rusqlite::types::FromSql for UserMetadata {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            match value {
                rusqlite::types::ValueRef::Text(items) => {
                    serde_json::from_slice(items)
                        .map_err(|_| rusqlite::types::FromSqlError::InvalidType)
                }
                _ => Err(rusqlite::types::FromSqlError::InvalidType),
            }
        }
    }
    impl rusqlite::types::ToSql for UserMetadata {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            let json = serde_json::to_string(self)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(json)))
        }
    }
    impl<'a> ::std::convert::TryInto<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UserMetadata {
        type Error = serde_json::Error;
        fn try_into(self) -> Result<::drizzle_rs::sqlite::SQLiteValue<'a>, Self::Error> {
            let json = serde_json::to_string(&self)?;
            Ok(::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Owned(json)))
        }
    }
    impl rusqlite::types::FromSql for UserConfig {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            match value {
                rusqlite::types::ValueRef::Blob(items) => {
                    serde_json::from_slice(items)
                        .map_err(|_| rusqlite::types::FromSqlError::InvalidType)
                }
                _ => Err(rusqlite::types::FromSqlError::InvalidType),
            }
        }
    }
    impl rusqlite::types::ToSql for UserConfig {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            let json = serde_json::to_vec(self)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Blob(json)))
        }
    }
    impl<'a> ::std::convert::TryInto<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UserConfig {
        type Error = serde_json::Error;
        fn try_into(self) -> Result<::drizzle_rs::sqlite::SQLiteValue<'a>, Self::Error> {
            let json = serde_json::to_vec(&self)?;
            Ok(::drizzle_rs::sqlite::SQLiteValue::Blob(::std::borrow::Cow::Owned(json)))
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectComplex {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                age: row.get("age")?,
                score: row.get("score")?,
                active: row.get("active")?,
                role: row.get("role")?,
                description: row.get("description")?,
                metadata: row.get("metadata")?,
                config: row.get("config")?,
                data_blob: row.get("data_blob")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectComplex {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                age: row.get("age")?,
                score: row.get("score")?,
                active: row.get("active")?,
                role: row.get("role")?,
                description: row.get("description")?,
                metadata: row.get("metadata")?,
                config: row.get("config")?,
                data_blob: row.get("data_blob")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdateComplex {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                age: row.get("age")?,
                score: row.get("score")?,
                active: row.get("active")?,
                role: row.get("role")?,
                description: row.get("description")?,
                metadata: row.get("metadata")?,
                config: row.get("config")?,
                data_blob: row.get("data_blob")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    pub struct Post {
        pub id: PostId,
        pub title: PostTitle,
        pub content: PostContent,
        pub author_id: PostAuthorId,
        pub published: PostPublished,
        pub tags: PostTags,
        pub created_at: PostCreatedAt,
    }
    #[automatically_derived]
    impl ::core::default::Default for Post {
        #[inline]
        fn default() -> Post {
            Post {
                id: ::core::default::Default::default(),
                title: ::core::default::Default::default(),
                content: ::core::default::Default::default(),
                author_id: ::core::default::Default::default(),
                published: ::core::default::Default::default(),
                tags: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Post {
        #[inline]
        fn clone(&self) -> Post {
            let _: ::core::clone::AssertParamIsClone<PostId>;
            let _: ::core::clone::AssertParamIsClone<PostTitle>;
            let _: ::core::clone::AssertParamIsClone<PostContent>;
            let _: ::core::clone::AssertParamIsClone<PostAuthorId>;
            let _: ::core::clone::AssertParamIsClone<PostPublished>;
            let _: ::core::clone::AssertParamIsClone<PostTags>;
            let _: ::core::clone::AssertParamIsClone<PostCreatedAt>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Post {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Post {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "title",
                "content",
                "author_id",
                "published",
                "tags",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.title,
                &self.content,
                &self.author_id,
                &self.published,
                &self.tags,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(f, "Post", names, values)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Post {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Post {
        #[inline]
        fn eq(&self, other: &Post) -> bool {
            self.id == other.id && self.title == other.title
                && self.content == other.content && self.author_id == other.author_id
                && self.published == other.published && self.tags == other.tags
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Post {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<PostId>;
            let _: ::core::cmp::AssertParamIsEq<PostTitle>;
            let _: ::core::cmp::AssertParamIsEq<PostContent>;
            let _: ::core::cmp::AssertParamIsEq<PostAuthorId>;
            let _: ::core::cmp::AssertParamIsEq<PostPublished>;
            let _: ::core::cmp::AssertParamIsEq<PostTags>;
            let _: ::core::cmp::AssertParamIsEq<PostCreatedAt>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Post {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.id, state);
            ::core::hash::Hash::hash(&self.title, state);
            ::core::hash::Hash::hash(&self.content, state);
            ::core::hash::Hash::hash(&self.author_id, state);
            ::core::hash::Hash::hash(&self.published, state);
            ::core::hash::Hash::hash(&self.tags, state);
            ::core::hash::Hash::hash(&self.created_at, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Post {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Post,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.id, &other.id) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(
                        &self.title,
                        &other.title,
                    ) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            match ::core::cmp::PartialOrd::partial_cmp(
                                &self.content,
                                &other.content,
                            ) {
                                ::core::option::Option::Some(
                                    ::core::cmp::Ordering::Equal,
                                ) => {
                                    match ::core::cmp::PartialOrd::partial_cmp(
                                        &self.author_id,
                                        &other.author_id,
                                    ) {
                                        ::core::option::Option::Some(
                                            ::core::cmp::Ordering::Equal,
                                        ) => {
                                            match ::core::cmp::PartialOrd::partial_cmp(
                                                &self.published,
                                                &other.published,
                                            ) {
                                                ::core::option::Option::Some(
                                                    ::core::cmp::Ordering::Equal,
                                                ) => {
                                                    match ::core::cmp::PartialOrd::partial_cmp(
                                                        &self.tags,
                                                        &other.tags,
                                                    ) {
                                                        ::core::option::Option::Some(
                                                            ::core::cmp::Ordering::Equal,
                                                        ) => {
                                                            ::core::cmp::PartialOrd::partial_cmp(
                                                                &self.created_at,
                                                                &other.created_at,
                                                            )
                                                        }
                                                        cmp => cmp,
                                                    }
                                                }
                                                cmp => cmp,
                                            }
                                        }
                                        cmp => cmp,
                                    }
                                }
                                cmp => cmp,
                            }
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Post {
        #[inline]
        fn cmp(&self, other: &Post) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.id, &other.id) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.title, &other.title) {
                        ::core::cmp::Ordering::Equal => {
                            match ::core::cmp::Ord::cmp(&self.content, &other.content) {
                                ::core::cmp::Ordering::Equal => {
                                    match ::core::cmp::Ord::cmp(
                                        &self.author_id,
                                        &other.author_id,
                                    ) {
                                        ::core::cmp::Ordering::Equal => {
                                            match ::core::cmp::Ord::cmp(
                                                &self.published,
                                                &other.published,
                                            ) {
                                                ::core::cmp::Ordering::Equal => {
                                                    match ::core::cmp::Ord::cmp(&self.tags, &other.tags) {
                                                        ::core::cmp::Ordering::Equal => {
                                                            ::core::cmp::Ord::cmp(&self.created_at, &other.created_at)
                                                        }
                                                        cmp => cmp,
                                                    }
                                                }
                                                cmp => cmp,
                                            }
                                        }
                                        cmp => cmp,
                                    }
                                }
                                cmp => cmp,
                            }
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[allow(non_upper_case_globals)]
    impl Post {
        const fn new() -> Self {
            Self {
                id: PostId::new(),
                title: PostTitle::new(),
                content: PostContent::new(),
                author_id: PostAuthorId::new(),
                published: PostPublished::new(),
                tags: PostTags::new(),
                created_at: PostCreatedAt::new(),
            }
        }
        pub const id: PostId = PostId;
        pub const title: PostTitle = PostTitle;
        pub const content: PostContent = PostContent;
        pub const author_id: PostAuthorId = PostAuthorId;
        pub const published: PostPublished = PostPublished;
        pub const tags: PostTags = PostTags;
        pub const created_at: PostCreatedAt = PostCreatedAt;
    }
    #[allow(non_camel_case_types)]
    pub struct PostId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostId {
        #[inline]
        fn clone(&self) -> PostId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostId {
        #[inline]
        fn default() -> PostId {
            PostId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostId {
        #[inline]
        fn cmp(&self, other: &PostId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostId {
        #[inline]
        fn eq(&self, other: &PostId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostId {
        const fn new() -> PostId {
            PostId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostId {
        const NAME: &'a str = "id";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "id INTEGER PRIMARY KEY NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostId {
        type Table = Post;
        type Type = i32;
        const PRIMARY_KEY: bool = true;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostId = PostId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for PostId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("id"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostTitle;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostTitle {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostTitle")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostTitle {
        #[inline]
        fn clone(&self) -> PostTitle {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostTitle {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostTitle {
        #[inline]
        fn default() -> PostTitle {
            PostTitle {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostTitle {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostTitle,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostTitle {
        #[inline]
        fn cmp(&self, other: &PostTitle) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostTitle {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostTitle {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostTitle {
        #[inline]
        fn eq(&self, other: &PostTitle) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostTitle {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostTitle {
        const fn new() -> PostTitle {
            PostTitle {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTitle {
        const NAME: &'a str = "title";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "title TEXT NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostTitle {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostTitle {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTitle {
        type Table = Post;
        type Type = String;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostTitle {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTitle {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostTitle = PostTitle::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for PostTitle {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("title"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostContent;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostContent {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostContent")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostContent {
        #[inline]
        fn clone(&self) -> PostContent {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostContent {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostContent {
        #[inline]
        fn default() -> PostContent {
            PostContent {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostContent {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostContent,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostContent {
        #[inline]
        fn cmp(&self, other: &PostContent) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostContent {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostContent {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostContent {
        #[inline]
        fn eq(&self, other: &PostContent) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostContent {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostContent {
        const fn new() -> PostContent {
            PostContent {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostContent {
        const NAME: &'a str = "content";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "content TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostContent {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostContent {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostContent {
        type Table = Post;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostContent {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostContent {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostContent = PostContent::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostContent {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("content"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostAuthorId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostAuthorId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostAuthorId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostAuthorId {
        #[inline]
        fn clone(&self) -> PostAuthorId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostAuthorId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostAuthorId {
        #[inline]
        fn default() -> PostAuthorId {
            PostAuthorId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostAuthorId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostAuthorId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostAuthorId {
        #[inline]
        fn cmp(&self, other: &PostAuthorId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostAuthorId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostAuthorId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostAuthorId {
        #[inline]
        fn eq(&self, other: &PostAuthorId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostAuthorId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostAuthorId {
        const fn new() -> PostAuthorId {
            PostAuthorId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostAuthorId {
        const NAME: &'a str = "author_id";
        const TYPE: &'a str = "BLOB";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "author_id BLOB",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostAuthorId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostAuthorId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostAuthorId {
        type Table = Post;
        type Type = Option<Uuid>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostAuthorId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostAuthorId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostAuthorId = PostAuthorId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostAuthorId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("author_id"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostPublished;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostPublished {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostPublished")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostPublished {
        #[inline]
        fn clone(&self) -> PostPublished {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostPublished {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostPublished {
        #[inline]
        fn default() -> PostPublished {
            PostPublished {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostPublished {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostPublished,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostPublished {
        #[inline]
        fn cmp(&self, other: &PostPublished) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostPublished {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostPublished {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostPublished {
        #[inline]
        fn eq(&self, other: &PostPublished) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostPublished {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostPublished {
        const fn new() -> PostPublished {
            PostPublished {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostPublished {
        const NAME: &'a str = "published";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "published INTEGER NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostPublished {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostPublished {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostPublished {
        type Table = Post;
        type Type = bool;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostPublished {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostPublished {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostPublished = PostPublished::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostPublished {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("published"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostTags;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostTags {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostTags")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostTags {
        #[inline]
        fn clone(&self) -> PostTags {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostTags {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostTags {
        #[inline]
        fn default() -> PostTags {
            PostTags {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostTags {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostTags,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostTags {
        #[inline]
        fn cmp(&self, other: &PostTags) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostTags {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostTags {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostTags {
        #[inline]
        fn eq(&self, other: &PostTags) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostTags {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostTags {
        const fn new() -> PostTags {
            PostTags {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTags {
        const NAME: &'a str = "tags";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "tags TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostTags {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostTags {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTags {
        type Table = Post;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostTags {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostTags {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostTags = PostTags::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for PostTags {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("tags"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostCreatedAt;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostCreatedAt {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostCreatedAt")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostCreatedAt {
        #[inline]
        fn clone(&self) -> PostCreatedAt {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostCreatedAt {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostCreatedAt {
        #[inline]
        fn default() -> PostCreatedAt {
            PostCreatedAt {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostCreatedAt {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostCreatedAt,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostCreatedAt {
        #[inline]
        fn cmp(&self, other: &PostCreatedAt) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostCreatedAt {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostCreatedAt {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostCreatedAt {
        #[inline]
        fn eq(&self, other: &PostCreatedAt) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostCreatedAt {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostCreatedAt {
        const fn new() -> PostCreatedAt {
            PostCreatedAt {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCreatedAt {
        const NAME: &'a str = "created_at";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "created_at TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostCreatedAt {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Post = Post::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostCreatedAt {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCreatedAt {
        type Table = Post;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostCreatedAt {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCreatedAt {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostCreatedAt = PostCreatedAt::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCreatedAt {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("created_at"),
            )
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<
        'a,
        ::drizzle_rs::core::SQLSchemaType,
        ::drizzle_rs::sqlite::SQLiteValue<'a>,
    > for Post {
        const NAME: &'a str = "posts";
        const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "CREATE TABLE \"posts\" (id INTEGER PRIMARY KEY NOT NULL, title TEXT NOT NULL, content TEXT, author_id BLOB, published INTEGER NOT NULL, tags TEXT, created_at TEXT);",
        );
    }
    impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Post {
        type Select = SelectPost;
        type Insert = InsertPost;
        type Update = UpdatePost;
    }
    impl ::drizzle_rs::core::SQLTableInfo for Post {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
            Self::TYPE
        }
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            #[allow(non_upper_case_globals)]
            static PostId: PostId = PostId::new();
            #[allow(non_upper_case_globals)]
            static PostTitle: PostTitle = PostTitle::new();
            #[allow(non_upper_case_globals)]
            static PostContent: PostContent = PostContent::new();
            #[allow(non_upper_case_globals)]
            static PostAuthorId: PostAuthorId = PostAuthorId::new();
            #[allow(non_upper_case_globals)]
            static PostPublished: PostPublished = PostPublished::new();
            #[allow(non_upper_case_globals)]
            static PostTags: PostTags = PostTags::new();
            #[allow(non_upper_case_globals)]
            static PostCreatedAt: PostCreatedAt = PostCreatedAt::new();
            Box::new([
                PostId.as_column(),
                PostTitle.as_column(),
                PostContent.as_column(),
                PostAuthorId.as_column(),
                PostPublished.as_column(),
                PostTags.as_column(),
                PostCreatedAt.as_column(),
            ])
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Post {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: Post = Post::new();
            INSTANCE.as_table().to_sql()
        }
    }
    pub struct SelectPost {
        pub id: i32,
        pub title: String,
        pub content: ::std::option::Option<String>,
        pub author_id: ::std::option::Option<Uuid>,
        pub published: bool,
        pub tags: ::std::option::Option<String>,
        pub created_at: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectPost {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "title",
                "content",
                "author_id",
                "published",
                "tags",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.title,
                &self.content,
                &self.author_id,
                &self.published,
                &self.tags,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "SelectPost",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SelectPost {
        #[inline]
        fn clone(&self) -> SelectPost {
            SelectPost {
                id: ::core::clone::Clone::clone(&self.id),
                title: ::core::clone::Clone::clone(&self.title),
                content: ::core::clone::Clone::clone(&self.content),
                author_id: ::core::clone::Clone::clone(&self.author_id),
                published: ::core::clone::Clone::clone(&self.published),
                tags: ::core::clone::Clone::clone(&self.tags),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for SelectPost {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for SelectPost {
        #[inline]
        fn eq(&self, other: &SelectPost) -> bool {
            self.id == other.id && self.published == other.published
                && self.title == other.title && self.content == other.content
                && self.author_id == other.author_id && self.tags == other.tags
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for SelectPost {
        #[inline]
        fn default() -> SelectPost {
            SelectPost {
                id: ::core::default::Default::default(),
                title: ::core::default::Default::default(),
                content: ::core::default::Default::default(),
                author_id: ::core::default::Default::default(),
                published: ::core::default::Default::default(),
                tags: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    pub struct PartialSelectPost {
        pub id: Option<i32>,
        pub title: Option<String>,
        pub content: Option<String>,
        pub author_id: Option<Uuid>,
        pub published: Option<bool>,
        pub tags: Option<String>,
        pub created_at: Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PartialSelectPost {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "title",
                "content",
                "author_id",
                "published",
                "tags",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.title,
                &self.content,
                &self.author_id,
                &self.published,
                &self.tags,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "PartialSelectPost",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PartialSelectPost {
        #[inline]
        fn clone(&self) -> PartialSelectPost {
            PartialSelectPost {
                id: ::core::clone::Clone::clone(&self.id),
                title: ::core::clone::Clone::clone(&self.title),
                content: ::core::clone::Clone::clone(&self.content),
                author_id: ::core::clone::Clone::clone(&self.author_id),
                published: ::core::clone::Clone::clone(&self.published),
                tags: ::core::clone::Clone::clone(&self.tags),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PartialSelectPost {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PartialSelectPost {
        #[inline]
        fn eq(&self, other: &PartialSelectPost) -> bool {
            self.id == other.id && self.title == other.title
                && self.content == other.content && self.author_id == other.author_id
                && self.published == other.published && self.tags == other.tags
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PartialSelectPost {
        #[inline]
        fn default() -> PartialSelectPost {
            PartialSelectPost {
                id: ::core::default::Default::default(),
                title: ::core::default::Default::default(),
                content: ::core::default::Default::default(),
                author_id: ::core::default::Default::default(),
                published: ::core::default::Default::default(),
                tags: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    impl PartialSelectPost {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_title<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.title = Some(value);
            self
        }
        pub fn with_content<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.content = Some(value);
            self
        }
        pub fn with_author_id<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
            let value = value.into();
            self.author_id = Some(value);
            self
        }
        pub fn with_published(mut self, value: bool) -> Self {
            self.published = Some(value);
            self
        }
        pub fn with_tags<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.tags = Some(value);
            self
        }
        pub fn with_created_at<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.created_at = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPost {
        type Partial = PartialSelectPost;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectPost {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPost {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct InsertPost {
        pub id: ::drizzle_rs::sqlite::InsertValue<i32>,
        pub title: ::drizzle_rs::sqlite::InsertValue<String>,
        pub content: ::drizzle_rs::sqlite::InsertValue<String>,
        pub author_id: ::drizzle_rs::sqlite::InsertValue<Uuid>,
        pub published: ::drizzle_rs::sqlite::InsertValue<bool>,
        pub tags: ::drizzle_rs::sqlite::InsertValue<String>,
        pub created_at: ::drizzle_rs::sqlite::InsertValue<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for InsertPost {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "title",
                "content",
                "author_id",
                "published",
                "tags",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.title,
                &self.content,
                &self.author_id,
                &self.published,
                &self.tags,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "InsertPost",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InsertPost {
        #[inline]
        fn clone(&self) -> InsertPost {
            InsertPost {
                id: ::core::clone::Clone::clone(&self.id),
                title: ::core::clone::Clone::clone(&self.title),
                content: ::core::clone::Clone::clone(&self.content),
                author_id: ::core::clone::Clone::clone(&self.author_id),
                published: ::core::clone::Clone::clone(&self.published),
                tags: ::core::clone::Clone::clone(&self.tags),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InsertPost {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InsertPost {
        #[inline]
        fn eq(&self, other: &InsertPost) -> bool {
            self.id == other.id && self.title == other.title
                && self.content == other.content && self.author_id == other.author_id
                && self.published == other.published && self.tags == other.tags
                && self.created_at == other.created_at
        }
    }
    impl Default for InsertPost {
        fn default() -> Self {
            Self {
                id: ::drizzle_rs::sqlite::InsertValue::Omit,
                title: ::drizzle_rs::sqlite::InsertValue::Omit,
                content: ::drizzle_rs::sqlite::InsertValue::Omit,
                author_id: ::drizzle_rs::sqlite::InsertValue::Omit,
                published: ::drizzle_rs::sqlite::InsertValue::Omit,
                tags: ::drizzle_rs::sqlite::InsertValue::Omit,
                created_at: ::drizzle_rs::sqlite::InsertValue::Omit,
            }
        }
    }
    impl InsertPost {
        pub fn new(title: impl Into<::std::string::String>, published: bool) -> Self {
            Self {
                title: ::drizzle_rs::sqlite::InsertValue::Value(title.into()),
                published: ::drizzle_rs::sqlite::InsertValue::Value(published),
                ..Self::default()
            }
        }
        pub fn with_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.id = value.into();
            self
        }
        pub fn with_title<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.title = value.into();
            self
        }
        pub fn with_content<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.content = value.into();
            self
        }
        pub fn with_author_id<V: Into<::drizzle_rs::sqlite::InsertValue<::uuid::Uuid>>>(
            mut self,
            value: V,
        ) -> Self {
            self.author_id = value.into();
            self
        }
        pub fn with_published<V: Into<::drizzle_rs::sqlite::InsertValue<bool>>>(
            mut self,
            value: V,
        ) -> Self {
            self.published = value.into();
            self
        }
        pub fn with_tags<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.tags = value.into();
            self
        }
        pub fn with_created_at<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.created_at = value.into();
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertPost {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.title {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.content {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.author_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.published {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.tags {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.created_at {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertPost {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static TABLE: Post = Post::new();
            let all_columns = TABLE.columns();
            let mut result_columns = Vec::new();
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.id {} else {
                result_columns.push(all_columns[0usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.title {} else {
                result_columns.push(all_columns[1usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.content {} else {
                result_columns.push(all_columns[2usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.author_id {} else {
                result_columns.push(all_columns[3usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.published {} else {
                result_columns.push(all_columns[4usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.tags {} else {
                result_columns.push(all_columns[5usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.created_at {} else {
                result_columns.push(all_columns[6usize]);
            }
            result_columns.into_boxed_slice()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.title {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.content {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.author_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.published {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.tags {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.created_at {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    pub struct UpdatePost {
        pub id: ::std::option::Option<i32>,
        pub title: ::std::option::Option<String>,
        pub content: ::std::option::Option<String>,
        pub author_id: ::std::option::Option<Uuid>,
        pub published: ::std::option::Option<bool>,
        pub tags: ::std::option::Option<String>,
        pub created_at: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdatePost {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "id",
                "title",
                "content",
                "author_id",
                "published",
                "tags",
                "created_at",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.id,
                &self.title,
                &self.content,
                &self.author_id,
                &self.published,
                &self.tags,
                &&self.created_at,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "UpdatePost",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UpdatePost {
        #[inline]
        fn clone(&self) -> UpdatePost {
            UpdatePost {
                id: ::core::clone::Clone::clone(&self.id),
                title: ::core::clone::Clone::clone(&self.title),
                content: ::core::clone::Clone::clone(&self.content),
                author_id: ::core::clone::Clone::clone(&self.author_id),
                published: ::core::clone::Clone::clone(&self.published),
                tags: ::core::clone::Clone::clone(&self.tags),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdatePost {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UpdatePost {
        #[inline]
        fn eq(&self, other: &UpdatePost) -> bool {
            self.id == other.id && self.title == other.title
                && self.content == other.content && self.author_id == other.author_id
                && self.published == other.published && self.tags == other.tags
                && self.created_at == other.created_at
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UpdatePost {
        #[inline]
        fn default() -> UpdatePost {
            UpdatePost {
                id: ::core::default::Default::default(),
                title: ::core::default::Default::default(),
                content: ::core::default::Default::default(),
                author_id: ::core::default::Default::default(),
                published: ::core::default::Default::default(),
                tags: ::core::default::Default::default(),
                created_at: ::core::default::Default::default(),
            }
        }
    }
    impl UpdatePost {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_title<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.title = Some(value);
            self
        }
        pub fn with_content<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.content = Some(value);
            self
        }
        pub fn with_author_id<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
            let value = value.into();
            self.author_id = Some(value);
            self
        }
        pub fn with_published(mut self, value: bool) -> Self {
            self.published = Some(value);
            self
        }
        pub fn with_tags<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.tags = Some(value);
            self
        }
        pub fn with_created_at<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.created_at = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdatePost {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut assignments = Vec::new();
            if let Some(val) = &self.id {
                assignments
                    .push((
                        "id",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.title {
                assignments
                    .push((
                        "title",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.content {
                assignments
                    .push((
                        "content",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.author_id {
                assignments
                    .push((
                        "author_id",
                        ::drizzle_rs::sqlite::SQLiteValue::Blob(
                            ::std::borrow::Cow::Owned(val.as_bytes().to_vec()),
                        ),
                    ));
            }
            if let Some(val) = &self.published {
                assignments
                    .push((
                        "published",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.tags {
                assignments
                    .push((
                        "tags",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.created_at {
                assignments
                    .push((
                        "created_at",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            ::drizzle_rs::core::SQL::assignments(assignments)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPost {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Post = Post::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdatePost {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Post = Post::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            if let Some(val) = &self.id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.title {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.content {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.author_id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.published {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.tags {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.created_at {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectPost {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Post = Post::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectPost {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                author_id: row.get("author_id")?,
                published: row.get("published")?,
                tags: row.get("tags")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectPost {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                author_id: row.get("author_id")?,
                published: row.get("published")?,
                tags: row.get("tags")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdatePost {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                author_id: row.get("author_id")?,
                published: row.get("published")?,
                tags: row.get("tags")?,
                created_at: row.get("created_at")?,
            })
        }
    }
    pub struct Category {
        pub id: CategoryId,
        pub name: CategoryName,
        pub description: CategoryDescription,
    }
    #[automatically_derived]
    impl ::core::default::Default for Category {
        #[inline]
        fn default() -> Category {
            Category {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Category {
        #[inline]
        fn clone(&self) -> Category {
            let _: ::core::clone::AssertParamIsClone<CategoryId>;
            let _: ::core::clone::AssertParamIsClone<CategoryName>;
            let _: ::core::clone::AssertParamIsClone<CategoryDescription>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Category {}
    #[automatically_derived]
    impl ::core::fmt::Debug for Category {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Category",
                "id",
                &self.id,
                "name",
                &self.name,
                "description",
                &&self.description,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Category {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Category {
        #[inline]
        fn eq(&self, other: &Category) -> bool {
            self.id == other.id && self.name == other.name
                && self.description == other.description
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Category {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<CategoryId>;
            let _: ::core::cmp::AssertParamIsEq<CategoryName>;
            let _: ::core::cmp::AssertParamIsEq<CategoryDescription>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Category {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.id, state);
            ::core::hash::Hash::hash(&self.name, state);
            ::core::hash::Hash::hash(&self.description, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Category {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Category,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.id, &other.id) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(&self.name, &other.name) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::cmp::PartialOrd::partial_cmp(
                                &self.description,
                                &other.description,
                            )
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Category {
        #[inline]
        fn cmp(&self, other: &Category) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.id, &other.id) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.name, &other.name) {
                        ::core::cmp::Ordering::Equal => {
                            ::core::cmp::Ord::cmp(&self.description, &other.description)
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[allow(non_upper_case_globals)]
    impl Category {
        const fn new() -> Self {
            Self {
                id: CategoryId::new(),
                name: CategoryName::new(),
                description: CategoryDescription::new(),
            }
        }
        pub const id: CategoryId = CategoryId;
        pub const name: CategoryName = CategoryName;
        pub const description: CategoryDescription = CategoryDescription;
    }
    #[allow(non_camel_case_types)]
    pub struct CategoryId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for CategoryId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "CategoryId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for CategoryId {
        #[inline]
        fn clone(&self) -> CategoryId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for CategoryId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for CategoryId {
        #[inline]
        fn default() -> CategoryId {
            CategoryId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for CategoryId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &CategoryId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for CategoryId {
        #[inline]
        fn cmp(&self, other: &CategoryId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for CategoryId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for CategoryId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for CategoryId {
        #[inline]
        fn eq(&self, other: &CategoryId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for CategoryId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl CategoryId {
        const fn new() -> CategoryId {
            CategoryId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryId {
        const NAME: &'a str = "id";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "id INTEGER PRIMARY KEY NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for CategoryId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Category = Category::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for CategoryId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryId {
        type Table = Category;
        type Type = i32;
        const PRIMARY_KEY: bool = true;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for CategoryId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: CategoryId = CategoryId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>> for CategoryId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("id"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct CategoryName;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for CategoryName {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "CategoryName")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for CategoryName {
        #[inline]
        fn clone(&self) -> CategoryName {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for CategoryName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for CategoryName {
        #[inline]
        fn default() -> CategoryName {
            CategoryName {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for CategoryName {
        #[inline]
        fn partial_cmp(
            &self,
            other: &CategoryName,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for CategoryName {
        #[inline]
        fn cmp(&self, other: &CategoryName) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for CategoryName {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for CategoryName {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for CategoryName {
        #[inline]
        fn eq(&self, other: &CategoryName) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for CategoryName {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl CategoryName {
        const fn new() -> CategoryName {
            CategoryName {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryName {
        const NAME: &'a str = "name";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "name TEXT NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for CategoryName {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Category = Category::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for CategoryName {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryName {
        type Table = Category;
        type Type = String;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for CategoryName {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryName {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: CategoryName = CategoryName::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryName {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(::std::borrow::Cow::Borrowed("name"))
        }
    }
    #[allow(non_camel_case_types)]
    pub struct CategoryDescription;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for CategoryDescription {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "CategoryDescription")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for CategoryDescription {
        #[inline]
        fn clone(&self) -> CategoryDescription {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for CategoryDescription {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for CategoryDescription {
        #[inline]
        fn default() -> CategoryDescription {
            CategoryDescription {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for CategoryDescription {
        #[inline]
        fn partial_cmp(
            &self,
            other: &CategoryDescription,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for CategoryDescription {
        #[inline]
        fn cmp(&self, other: &CategoryDescription) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for CategoryDescription {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for CategoryDescription {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for CategoryDescription {
        #[inline]
        fn eq(&self, other: &CategoryDescription) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for CategoryDescription {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl CategoryDescription {
        const fn new() -> CategoryDescription {
            CategoryDescription {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryDescription {
        const NAME: &'a str = "description";
        const TYPE: &'a str = "TEXT";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "description TEXT",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for CategoryDescription {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: Category = Category::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for CategoryDescription {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryDescription {
        type Table = Category;
        type Type = Option<String>;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = false;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for CategoryDescription {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryDescription {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: CategoryDescription = CategoryDescription::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for CategoryDescription {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("description"),
            )
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<
        'a,
        ::drizzle_rs::core::SQLSchemaType,
        ::drizzle_rs::sqlite::SQLiteValue<'a>,
    > for Category {
        const NAME: &'a str = "categories";
        const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "CREATE TABLE \"categories\" (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, description TEXT);",
        );
    }
    impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Category {
        type Select = SelectCategory;
        type Insert = InsertCategory;
        type Update = UpdateCategory;
    }
    impl ::drizzle_rs::core::SQLTableInfo for Category {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
            Self::TYPE
        }
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            #[allow(non_upper_case_globals)]
            static CategoryId: CategoryId = CategoryId::new();
            #[allow(non_upper_case_globals)]
            static CategoryName: CategoryName = CategoryName::new();
            #[allow(non_upper_case_globals)]
            static CategoryDescription: CategoryDescription = CategoryDescription::new();
            Box::new([
                CategoryId.as_column(),
                CategoryName.as_column(),
                CategoryDescription.as_column(),
            ])
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for Category {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: Category = Category::new();
            INSTANCE.as_table().to_sql()
        }
    }
    pub struct SelectCategory {
        pub id: i32,
        pub name: String,
        pub description: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "SelectCategory",
                "id",
                &self.id,
                "name",
                &self.name,
                "description",
                &&self.description,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SelectCategory {
        #[inline]
        fn clone(&self) -> SelectCategory {
            SelectCategory {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                description: ::core::clone::Clone::clone(&self.description),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for SelectCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for SelectCategory {
        #[inline]
        fn eq(&self, other: &SelectCategory) -> bool {
            self.id == other.id && self.name == other.name
                && self.description == other.description
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for SelectCategory {
        #[inline]
        fn default() -> SelectCategory {
            SelectCategory {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
            }
        }
    }
    pub struct PartialSelectCategory {
        pub id: Option<i32>,
        pub name: Option<String>,
        pub description: Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PartialSelectCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "PartialSelectCategory",
                "id",
                &self.id,
                "name",
                &self.name,
                "description",
                &&self.description,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PartialSelectCategory {
        #[inline]
        fn clone(&self) -> PartialSelectCategory {
            PartialSelectCategory {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                description: ::core::clone::Clone::clone(&self.description),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PartialSelectCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PartialSelectCategory {
        #[inline]
        fn eq(&self, other: &PartialSelectCategory) -> bool {
            self.id == other.id && self.name == other.name
                && self.description == other.description
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PartialSelectCategory {
        #[inline]
        fn default() -> PartialSelectCategory {
            PartialSelectCategory {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
            }
        }
    }
    impl PartialSelectCategory {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
        pub fn with_description<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.description = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectCategory {
        type Partial = PartialSelectCategory;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct InsertCategory {
        pub id: ::drizzle_rs::sqlite::InsertValue<i32>,
        pub name: ::drizzle_rs::sqlite::InsertValue<String>,
        pub description: ::drizzle_rs::sqlite::InsertValue<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for InsertCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "InsertCategory",
                "id",
                &self.id,
                "name",
                &self.name,
                "description",
                &&self.description,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InsertCategory {
        #[inline]
        fn clone(&self) -> InsertCategory {
            InsertCategory {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                description: ::core::clone::Clone::clone(&self.description),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InsertCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InsertCategory {
        #[inline]
        fn eq(&self, other: &InsertCategory) -> bool {
            self.id == other.id && self.name == other.name
                && self.description == other.description
        }
    }
    impl Default for InsertCategory {
        fn default() -> Self {
            Self {
                id: ::drizzle_rs::sqlite::InsertValue::Omit,
                name: ::drizzle_rs::sqlite::InsertValue::Omit,
                description: ::drizzle_rs::sqlite::InsertValue::Omit,
            }
        }
    }
    impl InsertCategory {
        pub fn new(name: impl Into<::std::string::String>) -> Self {
            Self {
                name: ::drizzle_rs::sqlite::InsertValue::Value(name.into()),
                ..Self::default()
            }
        }
        pub fn with_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.id = value.into();
            self
        }
        pub fn with_name<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.name = value.into();
            self
        }
        pub fn with_description<
            V: Into<::drizzle_rs::sqlite::InsertValue<::std::string::String>>,
        >(mut self, value: V) -> Self {
            self.description = value.into();
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.description {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static TABLE: Category = Category::new();
            let all_columns = TABLE.columns();
            let mut result_columns = Vec::new();
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.id {} else {
                result_columns.push(all_columns[0usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.name {} else {
                result_columns.push(all_columns[1usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.description {} else {
                result_columns.push(all_columns[2usize]);
            }
            result_columns.into_boxed_slice()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.name {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.description {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    pub struct UpdateCategory {
        pub id: ::std::option::Option<i32>,
        pub name: ::std::option::Option<String>,
        pub description: ::std::option::Option<String>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdateCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "UpdateCategory",
                "id",
                &self.id,
                "name",
                &self.name,
                "description",
                &&self.description,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UpdateCategory {
        #[inline]
        fn clone(&self) -> UpdateCategory {
            UpdateCategory {
                id: ::core::clone::Clone::clone(&self.id),
                name: ::core::clone::Clone::clone(&self.name),
                description: ::core::clone::Clone::clone(&self.description),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdateCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UpdateCategory {
        #[inline]
        fn eq(&self, other: &UpdateCategory) -> bool {
            self.id == other.id && self.name == other.name
                && self.description == other.description
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UpdateCategory {
        #[inline]
        fn default() -> UpdateCategory {
            UpdateCategory {
                id: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                description: ::core::default::Default::default(),
            }
        }
    }
    impl UpdateCategory {
        pub fn with_id(mut self, value: i32) -> Self {
            self.id = Some(value);
            self
        }
        pub fn with_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
            let value = value.into();
            self.name = Some(value);
            self
        }
        pub fn with_description<T: Into<::std::string::String>>(
            mut self,
            value: T,
        ) -> Self {
            let value = value.into();
            self.description = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut assignments = Vec::new();
            if let Some(val) = &self.id {
                assignments
                    .push((
                        "id",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.name {
                assignments
                    .push((
                        "name",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.description {
                assignments
                    .push((
                        "description",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            ::drizzle_rs::core::SQL::assignments(assignments)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Category = Category::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdateCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Category = Category::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            if let Some(val) = &self.id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.name {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.description {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: Category = Category::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdateCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
            })
        }
    }
    pub struct PostCategory {
        pub post_id: PostCategoryPostId,
        pub category_id: PostCategoryCategoryId,
    }
    #[automatically_derived]
    impl ::core::default::Default for PostCategory {
        #[inline]
        fn default() -> PostCategory {
            PostCategory {
                post_id: ::core::default::Default::default(),
                category_id: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PostCategory {
        #[inline]
        fn clone(&self) -> PostCategory {
            let _: ::core::clone::AssertParamIsClone<PostCategoryPostId>;
            let _: ::core::clone::AssertParamIsClone<PostCategoryCategoryId>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for PostCategory {}
    #[automatically_derived]
    impl ::core::fmt::Debug for PostCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostCategory",
                "post_id",
                &self.post_id,
                "category_id",
                &&self.category_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PostCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PostCategory {
        #[inline]
        fn eq(&self, other: &PostCategory) -> bool {
            self.post_id == other.post_id && self.category_id == other.category_id
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for PostCategory {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<PostCategoryPostId>;
            let _: ::core::cmp::AssertParamIsEq<PostCategoryCategoryId>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for PostCategory {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.post_id, state);
            ::core::hash::Hash::hash(&self.category_id, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for PostCategory {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostCategory,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.post_id, &other.post_id) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    ::core::cmp::PartialOrd::partial_cmp(
                        &self.category_id,
                        &other.category_id,
                    )
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for PostCategory {
        #[inline]
        fn cmp(&self, other: &PostCategory) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.post_id, &other.post_id) {
                ::core::cmp::Ordering::Equal => {
                    ::core::cmp::Ord::cmp(&self.category_id, &other.category_id)
                }
                cmp => cmp,
            }
        }
    }
    #[allow(non_upper_case_globals)]
    impl PostCategory {
        const fn new() -> Self {
            Self {
                post_id: PostCategoryPostId::new(),
                category_id: PostCategoryCategoryId::new(),
            }
        }
        pub const post_id: PostCategoryPostId = PostCategoryPostId;
        pub const category_id: PostCategoryCategoryId = PostCategoryCategoryId;
    }
    #[allow(non_camel_case_types)]
    pub struct PostCategoryPostId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostCategoryPostId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostCategoryPostId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostCategoryPostId {
        #[inline]
        fn clone(&self) -> PostCategoryPostId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostCategoryPostId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostCategoryPostId {
        #[inline]
        fn default() -> PostCategoryPostId {
            PostCategoryPostId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostCategoryPostId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostCategoryPostId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostCategoryPostId {
        #[inline]
        fn cmp(&self, other: &PostCategoryPostId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostCategoryPostId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostCategoryPostId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostCategoryPostId {
        #[inline]
        fn eq(&self, other: &PostCategoryPostId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostCategoryPostId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostCategoryPostId {
        const fn new() -> PostCategoryPostId {
            PostCategoryPostId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryPostId {
        const NAME: &'a str = "post_id";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "post_id INTEGER NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostCategoryPostId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: PostCategory = PostCategory::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostCategoryPostId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryPostId {
        type Table = PostCategory;
        type Type = i32;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostCategoryPostId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryPostId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostCategoryPostId = PostCategoryPostId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryPostId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("post_id"),
            )
        }
    }
    #[allow(non_camel_case_types)]
    pub struct PostCategoryCategoryId;
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for PostCategoryCategoryId {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PostCategoryCategoryId")
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::clone::Clone for PostCategoryCategoryId {
        #[inline]
        fn clone(&self) -> PostCategoryCategoryId {
            *self
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::Copy for PostCategoryCategoryId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::default::Default for PostCategoryCategoryId {
        #[inline]
        fn default() -> PostCategoryCategoryId {
            PostCategoryCategoryId {}
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialOrd for PostCategoryCategoryId {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PostCategoryCategoryId,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Ord for PostCategoryCategoryId {
        #[inline]
        fn cmp(&self, other: &PostCategoryCategoryId) -> ::core::cmp::Ordering {
            ::core::cmp::Ordering::Equal
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::Eq for PostCategoryCategoryId {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::marker::StructuralPartialEq for PostCategoryCategoryId {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::cmp::PartialEq for PostCategoryCategoryId {
        #[inline]
        fn eq(&self, other: &PostCategoryCategoryId) -> bool {
            true
        }
    }
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::hash::Hash for PostCategoryCategoryId {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {}
    }
    impl PostCategoryCategoryId {
        const fn new() -> PostCategoryCategoryId {
            PostCategoryCategoryId {}
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<'a, &'a str, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryCategoryId {
        const NAME: &'a str = "category_id";
        const TYPE: &'a str = "INTEGER";
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "category_id INTEGER NOT NULL",
        );
    }
    impl ::drizzle_rs::core::SQLColumnInfo for PostCategoryCategoryId {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> &str {
            Self::TYPE
        }
        fn is_primary_key(&self) -> bool {
            Self::PRIMARY_KEY
        }
        fn is_not_null(&self) -> bool {
            Self::NOT_NULL
        }
        fn is_unique(&self) -> bool {
            Self::UNIQUE
        }
        fn has_default(&self) -> bool {
            false
        }
        fn table(&self) -> &dyn SQLTableInfo {
            static TABLE: PostCategory = PostCategory::new();
            &TABLE
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumnInfo for PostCategoryCategoryId {
        fn is_autoincrement(&self) -> bool {
            <Self as ::drizzle_rs::sqlite::SQLiteColumn<'_>>::AUTOINCREMENT
        }
    }
    impl<'a> ::drizzle_rs::core::SQLColumn<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryCategoryId {
        type Table = PostCategory;
        type Type = i32;
        const PRIMARY_KEY: bool = false;
        const NOT_NULL: bool = true;
        const UNIQUE: bool = false;
        const DEFAULT: Option<Self::Type> = None;
        fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
            None::<fn() -> Self::Type>
        }
    }
    impl ::drizzle_rs::sqlite::SQLiteColumn<'_> for PostCategoryCategoryId {
        const AUTOINCREMENT: bool = false;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryCategoryId {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostCategoryCategoryId = PostCategoryCategoryId::new();
            INSTANCE.as_column().to_sql()
        }
    }
    impl<'a> ::std::convert::Into<::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategoryCategoryId {
        fn into(self) -> ::drizzle_rs::sqlite::SQLiteValue<'a> {
            ::drizzle_rs::sqlite::SQLiteValue::Text(
                ::std::borrow::Cow::Borrowed("category_id"),
            )
        }
    }
    impl<
        'a,
    > ::drizzle_rs::core::SQLSchema<
        'a,
        ::drizzle_rs::core::SQLSchemaType,
        ::drizzle_rs::sqlite::SQLiteValue<'a>,
    > for PostCategory {
        const NAME: &'a str = "post_categories";
        const TYPE: ::drizzle_rs::core::SQLSchemaType = ::drizzle_rs::core::SQLSchemaType::Table;
        const SQL: ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> = ::drizzle_rs::core::SQL::text(
            "CREATE TABLE \"post_categories\" (post_id INTEGER NOT NULL, category_id INTEGER NOT NULL);",
        );
    }
    impl<'a> ::drizzle_rs::core::SQLTable<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategory {
        type Select = SelectPostCategory;
        type Insert = InsertPostCategory;
        type Update = UpdatePostCategory;
    }
    impl ::drizzle_rs::core::SQLTableInfo for PostCategory {
        fn name(&self) -> &str {
            Self::NAME
        }
        fn r#type(&self) -> ::drizzle_rs::core::SQLSchemaType {
            Self::TYPE
        }
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            #[allow(non_upper_case_globals)]
            static PostCategoryPostId: PostCategoryPostId = PostCategoryPostId::new();
            #[allow(non_upper_case_globals)]
            static PostCategoryCategoryId: PostCategoryCategoryId = PostCategoryCategoryId::new();
            Box::new([
                PostCategoryPostId.as_column(),
                PostCategoryCategoryId.as_column(),
            ])
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PostCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            use ::drizzle_rs::core::ToSQL;
            static INSTANCE: PostCategory = PostCategory::new();
            INSTANCE.as_table().to_sql()
        }
    }
    pub struct SelectPostCategory {
        pub post_id: i32,
        pub category_id: i32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectPostCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "SelectPostCategory",
                "post_id",
                &self.post_id,
                "category_id",
                &&self.category_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SelectPostCategory {
        #[inline]
        fn clone(&self) -> SelectPostCategory {
            SelectPostCategory {
                post_id: ::core::clone::Clone::clone(&self.post_id),
                category_id: ::core::clone::Clone::clone(&self.category_id),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for SelectPostCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for SelectPostCategory {
        #[inline]
        fn eq(&self, other: &SelectPostCategory) -> bool {
            self.post_id == other.post_id && self.category_id == other.category_id
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for SelectPostCategory {
        #[inline]
        fn default() -> SelectPostCategory {
            SelectPostCategory {
                post_id: ::core::default::Default::default(),
                category_id: ::core::default::Default::default(),
            }
        }
    }
    pub struct PartialSelectPostCategory {
        pub post_id: Option<i32>,
        pub category_id: Option<i32>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PartialSelectPostCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PartialSelectPostCategory",
                "post_id",
                &self.post_id,
                "category_id",
                &&self.category_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PartialSelectPostCategory {
        #[inline]
        fn clone(&self) -> PartialSelectPostCategory {
            PartialSelectPostCategory {
                post_id: ::core::clone::Clone::clone(&self.post_id),
                category_id: ::core::clone::Clone::clone(&self.category_id),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PartialSelectPostCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PartialSelectPostCategory {
        #[inline]
        fn eq(&self, other: &PartialSelectPostCategory) -> bool {
            self.post_id == other.post_id && self.category_id == other.category_id
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PartialSelectPostCategory {
        #[inline]
        fn default() -> PartialSelectPostCategory {
            PartialSelectPostCategory {
                post_id: ::core::default::Default::default(),
                category_id: ::core::default::Default::default(),
            }
        }
    }
    impl PartialSelectPostCategory {
        pub fn with_post_id(mut self, value: i32) -> Self {
            self.post_id = Some(value);
            self
        }
        pub fn with_category_id(mut self, value: i32) -> Self {
            self.category_id = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPostCategory {
        type Partial = PartialSelectPostCategory;
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectPostCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPostCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::core::panicking::panic("not implemented")
        }
    }
    pub struct InsertPostCategory {
        pub post_id: ::drizzle_rs::sqlite::InsertValue<i32>,
        pub category_id: ::drizzle_rs::sqlite::InsertValue<i32>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for InsertPostCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "InsertPostCategory",
                "post_id",
                &self.post_id,
                "category_id",
                &&self.category_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InsertPostCategory {
        #[inline]
        fn clone(&self) -> InsertPostCategory {
            InsertPostCategory {
                post_id: ::core::clone::Clone::clone(&self.post_id),
                category_id: ::core::clone::Clone::clone(&self.category_id),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for InsertPostCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for InsertPostCategory {
        #[inline]
        fn eq(&self, other: &InsertPostCategory) -> bool {
            self.post_id == other.post_id && self.category_id == other.category_id
        }
    }
    impl Default for InsertPostCategory {
        fn default() -> Self {
            Self {
                post_id: ::drizzle_rs::sqlite::InsertValue::Omit,
                category_id: ::drizzle_rs::sqlite::InsertValue::Omit,
            }
        }
    }
    impl InsertPostCategory {
        pub fn new(post_id: i32, category_id: i32) -> Self {
            Self {
                post_id: ::drizzle_rs::sqlite::InsertValue::Value(post_id),
                category_id: ::drizzle_rs::sqlite::InsertValue::Value(category_id),
                ..Self::default()
            }
        }
        pub fn with_post_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.post_id = value.into();
            self
        }
        pub fn with_category_id<V: Into<::drizzle_rs::sqlite::InsertValue<i32>>>(
            mut self,
            value: V,
        ) -> Self {
            self.category_id = value.into();
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertPostCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.post_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.category_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for InsertPostCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static TABLE: PostCategory = PostCategory::new();
            let all_columns = TABLE.columns();
            let mut result_columns = Vec::new();
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.post_id {} else {
                result_columns.push(all_columns[0usize]);
            }
            if let ::drizzle_rs::sqlite::InsertValue::Omit = &self.category_id {} else {
                result_columns.push(all_columns[1usize]);
            }
            result_columns.into_boxed_slice()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            match &self.post_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            match &self.category_id {
                ::drizzle_rs::sqlite::InsertValue::Omit => {}
                ::drizzle_rs::sqlite::InsertValue::Null => {
                    values.push(::drizzle_rs::sqlite::SQLiteValue::Null);
                }
                ::drizzle_rs::sqlite::InsertValue::Value(val) => {
                    values
                        .push(
                            val
                                .clone()
                                .try_into()
                                .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                        );
                }
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    pub struct UpdatePostCategory {
        pub post_id: ::std::option::Option<i32>,
        pub category_id: ::std::option::Option<i32>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdatePostCategory {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "UpdatePostCategory",
                "post_id",
                &self.post_id,
                "category_id",
                &&self.category_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UpdatePostCategory {
        #[inline]
        fn clone(&self) -> UpdatePostCategory {
            UpdatePostCategory {
                post_id: ::core::clone::Clone::clone(&self.post_id),
                category_id: ::core::clone::Clone::clone(&self.category_id),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UpdatePostCategory {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UpdatePostCategory {
        #[inline]
        fn eq(&self, other: &UpdatePostCategory) -> bool {
            self.post_id == other.post_id && self.category_id == other.category_id
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UpdatePostCategory {
        #[inline]
        fn default() -> UpdatePostCategory {
            UpdatePostCategory {
                post_id: ::core::default::Default::default(),
                category_id: ::core::default::Default::default(),
            }
        }
    }
    impl UpdatePostCategory {
        pub fn with_post_id(mut self, value: i32) -> Self {
            self.post_id = Some(value);
            self
        }
        pub fn with_category_id(mut self, value: i32) -> Self {
            self.category_id = Some(value);
            self
        }
    }
    impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdatePostCategory {
        fn to_sql(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut assignments = Vec::new();
            if let Some(val) = &self.post_id {
                assignments
                    .push((
                        "post_id",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            if let Some(val) = &self.category_id {
                assignments
                    .push((
                        "category_id",
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    ));
            }
            ::drizzle_rs::core::SQL::assignments(assignments)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for SelectPostCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: PostCategory = PostCategory::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for UpdatePostCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: PostCategory = PostCategory::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            let mut values = Vec::new();
            if let Some(val) = &self.post_id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            if let Some(val) = &self.category_id {
                values
                    .push(
                        val
                            .clone()
                            .try_into()
                            .unwrap_or(::drizzle_rs::sqlite::SQLiteValue::Null),
                    );
            }
            ::drizzle_rs::core::SQL::parameters(values)
        }
    }
    impl<'a> ::drizzle_rs::core::SQLModel<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>>
    for PartialSelectPostCategory {
        fn columns(&self) -> Box<[&'static dyn ::drizzle_rs::core::SQLColumnInfo]> {
            static INSTANCE: PostCategory = PostCategory::new();
            INSTANCE.columns()
        }
        fn values(
            &self,
        ) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
            ::drizzle_rs::core::SQL::empty()
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for SelectPostCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                post_id: row.get("post_id")?,
                category_id: row.get("category_id")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for PartialSelectPostCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                post_id: row.get("post_id")?,
                category_id: row.get("category_id")?,
            })
        }
    }
    impl ::std::convert::TryFrom<&rusqlite::Row<'_>> for UpdatePostCategory {
        type Error = ::rusqlite::Error;
        fn try_from(
            row: &::rusqlite::Row<'_>,
        ) -> ::std::result::Result<Self, Self::Error> {
            Ok(Self {
                post_id: row.get("post_id")?,
                category_id: row.get("category_id")?,
            })
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_prepare_with_placeholder"]
#[doc(hidden)]
pub const test_prepare_with_placeholder: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_with_placeholder"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 25usize,
        start_col: 4usize,
        end_line: 25usize,
        end_col: 33usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_with_placeholder()),
    ),
};
fn test_prepare_with_placeholder() {
    let conn = setup_db();
    let (db, (simple, complex)) = {
        #[allow(non_camel_case_types)]
        pub struct SimpleComplexSchema;
        #[automatically_derived]
        #[allow(non_camel_case_types)]
        impl ::core::clone::Clone for SimpleComplexSchema {
            #[inline]
            fn clone(&self) -> SimpleComplexSchema {
                SimpleComplexSchema
            }
        }
        #[automatically_derived]
        #[allow(non_camel_case_types)]
        impl ::core::fmt::Debug for SimpleComplexSchema {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "SimpleComplexSchema")
            }
        }
        impl ::drizzle_rs::core::IsInSchema<SimpleComplexSchema> for Simple {}
        impl ::drizzle_rs::core::IsInSchema<SimpleComplexSchema> for Complex {}
        (
            ::drizzle_rs::sqlite::Drizzle::new::<SimpleComplexSchema>(conn),
            (Simple::default(), Complex::default()),
        )
    };
    let effected = db
        .insert(simple)
        .values([InsertSimple::new("alice")])
        .execute()
        .unwrap();
    let prepared_sql = db
        .select(simple.name)
        .from(simple)
        .r#where(eq(simple.name, SQL::placeholder("name")))
        .prepare();
    {
        ::std::io::_print(format_args!("{0}\n", prepared_sql));
    };
    let result: Vec<PartialSelectSimple> = prepared_sql
        .all([
            ::drizzle_rs::core::ParamBind::new(
                "name",
                ::sqlite::SQLiteValue::from("alice"),
            ),
        ])
        .unwrap();
    match (&result.len(), &1) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&result[0].name, &Some("alice".into())) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_prepare_render_basic"]
#[doc(hidden)]
pub const test_prepare_render_basic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_render_basic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 52usize,
        start_col: 4usize,
        end_line: 52usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_render_basic()),
    ),
};
fn test_prepare_render_basic() {
    let sql = SQL::<SQLiteValue>::raw("SELECT * FROM users WHERE id = ")
        .append(SQL::placeholder("user_id"))
        .append_raw(" AND name = ")
        .append(SQL::placeholder("user_name"));
    let prepared = sql.prepare_render();
    match (&prepared.text_segments.len(), &3) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.params.len(), &2) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.text_segments[0], &"SELECT * FROM users WHERE id = ") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.text_segments[1], &" AND name = ") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.text_segments[2], &"") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.params.len(), &2) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_prepare_with_multiple_parameters"]
#[doc(hidden)]
pub const test_prepare_with_multiple_parameters: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_with_multiple_parameters"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 75usize,
        start_col: 4usize,
        end_line: 75usize,
        end_col: 41usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_with_multiple_parameters()),
    ),
};
fn test_prepare_with_multiple_parameters() {
    let sql = SQL::<SQLiteValue>::raw("INSERT INTO users (name, age, active) VALUES (")
        .append(SQL::placeholder("name"))
        .append_raw(", ")
        .append(SQL::placeholder("age"))
        .append_raw(", ")
        .append(SQL::placeholder("active"))
        .append_raw(")");
    let prepared = sql.prepare_render();
    let (final_sql, bound_params) = prepared
        .bind([
            ::drizzle_rs::core::ParamBind::new(
                "name",
                ::sqlite::SQLiteValue::from("alice"),
            ),
            ::drizzle_rs::core::ParamBind::new(
                "age",
                ::sqlite::SQLiteValue::from(25i32),
            ),
            ::drizzle_rs::core::ParamBind::new(
                "active",
                ::sqlite::SQLiteValue::from(true),
            ),
        ]);
    if !final_sql.contains("INSERT INTO users (name, age, active) VALUES (") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\"INSERT INTO users (name, age, active) VALUES (\")",
        )
    }
    if !final_sql.contains(":name") {
        ::core::panicking::panic("assertion failed: final_sql.contains(\":name\")")
    }
    if !final_sql.contains(":age") {
        ::core::panicking::panic("assertion failed: final_sql.contains(\":age\")")
    }
    if !final_sql.contains(":active") {
        ::core::panicking::panic("assertion failed: final_sql.contains(\":active\")")
    }
    match (&bound_params.len(), &3) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params[0], &SQLiteValue::from("alice")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params[1], &SQLiteValue::from(25i32)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params[2], &SQLiteValue::from(true)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_prepare_sql_reconstruction"]
#[doc(hidden)]
pub const test_prepare_sql_reconstruction: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_sql_reconstruction"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 106usize,
        start_col: 4usize,
        end_line: 106usize,
        end_col: 35usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_sql_reconstruction()),
    ),
};
fn test_prepare_sql_reconstruction() {
    let _original_query = "SELECT * FROM posts WHERE author = :author AND published = :published ORDER BY created_at DESC";
    let sql = SQL::<SQLiteValue>::raw("SELECT * FROM posts WHERE author = ")
        .append(SQL::placeholder("author"))
        .append_raw(" AND published = ")
        .append(SQL::placeholder("published"))
        .append_raw(" ORDER BY created_at DESC");
    let prepared = sql.prepare_render();
    let (final_sql, _) = prepared
        .bind([
            ::drizzle_rs::core::ParamBind::new(
                "author",
                ::sqlite::SQLiteValue::from("john_doe"),
            ),
            ::drizzle_rs::core::ParamBind::new(
                "published",
                ::sqlite::SQLiteValue::from(true),
            ),
        ]);
    if !final_sql.contains("SELECT * FROM posts WHERE author = :author") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\"SELECT * FROM posts WHERE author = :author\")",
        )
    }
    if !final_sql.contains("AND published = :published") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\"AND published = :published\")",
        )
    }
    if !final_sql.contains("ORDER BY created_at DESC") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\"ORDER BY created_at DESC\")",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_prepare_with_no_parameters"]
#[doc(hidden)]
pub const test_prepare_with_no_parameters: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_with_no_parameters"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 129usize,
        start_col: 4usize,
        end_line: 129usize,
        end_col: 35usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_with_no_parameters()),
    ),
};
fn test_prepare_with_no_parameters() {
    let sql = SQL::<SQLiteValue>::raw("SELECT COUNT(*) FROM users");
    let prepared = sql.prepare_render();
    match (&prepared.text_segments.len(), &1) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.params.len(), &0) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prepared.text_segments[0], &"SELECT COUNT(*) FROM users") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let (final_sql, bound_params) = prepared.bind([]);
    match (&final_sql, &"SELECT COUNT(*) FROM users") {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params.len(), &0) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_prepare_complex_query"]
#[doc(hidden)]
pub const test_prepare_complex_query: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prepare_complex_query"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\prepare.rs",
        start_line: 145usize,
        start_col: 4usize,
        end_line: 145usize,
        end_col: 30usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prepare_complex_query()),
    ),
};
fn test_prepare_complex_query() {
    let sql = SQL::<SQLiteValue>::raw("WITH RECURSIVE category_tree AS (")
        .append_raw("SELECT id, name, parent_id FROM categories WHERE id = ")
        .append(SQL::placeholder("root_id"))
        .append_raw(" UNION ALL SELECT c.id, c.name, c.parent_id FROM categories c ")
        .append_raw("INNER JOIN category_tree ct ON c.parent_id = ct.id) ")
        .append_raw("SELECT * FROM category_tree WHERE name LIKE ")
        .append(SQL::placeholder("search_pattern"));
    let prepared = sql.prepare_render();
    let (final_sql, bound_params) = prepared
        .bind([
            ::drizzle_rs::core::ParamBind::new(
                "root_id",
                ::sqlite::SQLiteValue::from(1i32),
            ),
            ::drizzle_rs::core::ParamBind::new(
                "search_pattern",
                ::sqlite::SQLiteValue::from("%electronics%"),
            ),
        ]);
    if !final_sql.contains("WITH RECURSIVE category_tree AS") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\"WITH RECURSIVE category_tree AS\")",
        )
    }
    if !final_sql.contains(":root_id") {
        ::core::panicking::panic("assertion failed: final_sql.contains(\":root_id\")")
    }
    if !final_sql.contains(":search_pattern") {
        ::core::panicking::panic(
            "assertion failed: final_sql.contains(\":search_pattern\")",
        )
    }
    match (&bound_params.len(), &2) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params[0], &SQLiteValue::from(1i32)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&bound_params[1], &SQLiteValue::from("%electronics%")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_prepare_complex_query,
            &test_prepare_render_basic,
            &test_prepare_sql_reconstruction,
            &test_prepare_with_multiple_parameters,
            &test_prepare_with_no_parameters,
            &test_prepare_with_placeholder,
        ],
    )
}
