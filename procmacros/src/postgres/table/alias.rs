use crate::paths::core as core_paths;
use crate::paths::std as std_paths;
use crate::postgres::table::context::MacroContext;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates an aliased version of a PostgreSQL table struct
///
/// For a table `Users` with fields `id` and `name`, this generates:
/// - `AliasedUsers` struct with `AliasedUsersId` and `AliasedUsersName` fields
/// - Each aliased field contains the table alias name
/// - `Users::alias::<Tag>() -> UsersAlias<Tag>` method
pub fn generate_aliased_table(ctx: &MacroContext) -> syn::Result<TokenStream> {
    let table_name = &ctx.struct_ident;
    let struct_vis = &ctx.struct_vis;
    let aliased_table_name = format_ident!("Aliased{}", table_name);
    let sql = core_paths::sql();
    let sql_column_info = core_paths::sql_column_info();
    let alias_tag = core_paths::tag();
    let taggable_alias = core_paths::taggable_alias();
    let phantom_data = std_paths::phantom_data();
    let token = core_paths::token();

    // Generate aliased field structs and their names
    let aliased_fields: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|field| {
            let field_name = &field.ident;
            // Use same casing as original column types to avoid conflicts
            let field_name_pascal = field_name.to_string().to_upper_camel_case();
            let aliased_field_type = format_ident!("Aliased{}{}", table_name, field_name_pascal);

            (field_name, aliased_field_type)
        })
        .collect();

    // Generate the aliased field type definitions
    let aliased_field_definitions: Vec<TokenStream> = ctx.field_infos.iter().zip(aliased_fields.iter()).map(|(field, (_, aliased_field_type))| {
        let field_name = &field.ident;
        // Use the same naming pattern as original column types
        let field_name_pascal = field_name.to_string().to_upper_camel_case();
        let original_field_type = format_ident!("{}{}", table_name, field_name_pascal);

        quote! {
            #[allow(non_upper_case_globals, dead_code)]
            #[derive(Debug, Clone, Copy, Default)]
            #struct_vis struct #aliased_field_type {
                alias: &'static str,
            }

            impl #aliased_field_type {
                pub const fn new(alias: &'static str) -> Self {
                    Self { alias }
                }
            }

            // Implement column info traits for the aliased field
            impl SQLColumnInfo for #aliased_field_type {
                fn is_not_null(&self) -> bool {
                    // Forward to the original field instance with explicit trait qualification
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_not_null(&ORIGINAL_FIELD)
                }

                fn is_primary_key(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_primary_key(&ORIGINAL_FIELD)
                }

                fn is_unique(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::is_unique(&ORIGINAL_FIELD)
                }

                fn name(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::name(&ORIGINAL_FIELD)
                }

                fn r#type(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::r#type(&ORIGINAL_FIELD)
                }

                fn has_default(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as SQLColumnInfo>::has_default(&ORIGINAL_FIELD)
                }

                fn table(&self) -> &dyn SQLTableInfo {
                    // This is tricky - we need a static reference but each column has different alias
                    // For now, return the original table info
                    static ORIGINAL_TABLE: #table_name = #table_name::new();
                    &ORIGINAL_TABLE
                }
            }

            // Implement PostgreSQL-specific column info traits
            impl PostgresColumnInfo for #aliased_field_type {
                fn is_serial(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_serial(&ORIGINAL_FIELD)
                }

                fn is_bigserial(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_bigserial(&ORIGINAL_FIELD)
                }

                fn is_generated_identity(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_generated_identity(&ORIGINAL_FIELD)
                }

                fn is_identity_always(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::is_identity_always(&ORIGINAL_FIELD)
                }

                fn postgres_type(&self) -> &'static str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::postgres_type(&ORIGINAL_FIELD)
                }

                fn table(&self) -> &dyn PostgresTableInfo {
                    // This is tricky - we need a static reference but each alias instance
                    // has a different alias string. For now, return the original table info.
                    static ORIGINAL_TABLE: #table_name = #table_name::new();
                    &ORIGINAL_TABLE
                }

                fn foreign_key(&self) -> Option<&'static dyn PostgresColumnInfo> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as PostgresColumnInfo>::foreign_key(&ORIGINAL_FIELD)
                }
            }

            // Implement SQLColumn trait for aliased field
            impl<'a> SQLColumn<'a, PostgresValue<'a>> for #aliased_field_type {
                type Table = #aliased_table_name;
                type TableType = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::TableType;
                type ForeignKeys = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::ForeignKeys;
                type Type = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::Type;

                const PRIMARY_KEY: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::PRIMARY_KEY;
                const NOT_NULL: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::NOT_NULL;
                const UNIQUE: bool = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::UNIQUE;
                const DEFAULT: Option<Self::Type> = <#original_field_type as SQLColumn<'a, PostgresValue<'a>>>::DEFAULT;

                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    ORIGINAL_FIELD.default_fn()
                }
            }

            // Implement PostgresColumn trait for aliased field
            impl<'a> PostgresColumn<'a> for #aliased_field_type {
                // PostgreSQL columns don't have autoincrement like SQLite, but may have other specific traits
            }

            // Implement SQLSchema trait for aliased field
            impl<'a> SQLSchema<'a, &'a str, PostgresValue<'a>> for #aliased_field_type {
                const NAME: &'static str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::NAME;
                const TYPE: &'a str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::TYPE;
                const SQL: &'static str = <#original_field_type as SQLSchema<'a, &'a str, PostgresValue<'a>>>::SQL;
            }
            // ToSQL implementation that uses the alias
            impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for #aliased_field_type {
                fn to_sql(&self) -> #sql<'a, V> {
                    #sql::ident(self.alias)
                        .push(#token::DOT)
                        .append(#sql::ident({
                            static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                            #sql_column_info::name(&ORIGINAL_FIELD)
                        }))
                }
            }

            // Expr impl inheriting types from original column
            impl<'a> drizzle::core::expr::Expr<'a, PostgresValue<'a>> for #aliased_field_type {
                type SQLType = <#original_field_type as drizzle::core::expr::Expr<'a, PostgresValue<'a>>>::SQLType;
                type Nullable = <#original_field_type as drizzle::core::expr::Expr<'a, PostgresValue<'a>>>::Nullable;
                type Aggregate = drizzle::core::expr::Scalar;
            }
            impl drizzle::core::ExprValueType for #aliased_field_type {
                type ValueType = <#original_field_type as drizzle::core::ExprValueType>::ValueType;
            }
            impl drizzle::core::IntoSelectTarget for #aliased_field_type {
                type Marker = drizzle::core::SelectCols<(#aliased_field_type,)>;
            }
        }
    }).collect();

    // Generate the aliased table struct fields
    let aliased_struct_fields: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                #struct_vis #field_name: #aliased_type
            }
        })
        .collect();

    // Generate field initializers for the alias() method
    let field_initializers: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                #field_name: #aliased_type::new(alias)
            }
        })
        .collect();

    let tagged_aliased_table_name = format_ident!("TaggedAliased{}", table_name);
    let alias_type_name = format_ident!("{}Alias", table_name);
    let tagged_const_defs: Vec<TokenStream> = aliased_fields
        .iter()
        .map(|(field_name, aliased_type)| {
            quote! {
                pub const #field_name: #aliased_type = #aliased_type::new(Tag::NAME);
            }
        })
        .collect();

    Ok(quote! {

        // Generate all aliased field type definitions
        #(#aliased_field_definitions)*

        // Generate the aliased table struct
        #[allow(non_upper_case_globals, dead_code)]
        #[derive(Debug, Clone, Copy, Default)]
        #struct_vis struct #aliased_table_name {
            alias: &'static str,
            #(#aliased_struct_fields),*
        }

        impl #aliased_table_name {
            pub const fn new(alias: &'static str) -> Self {
                Self {
                    alias,
                    #(#field_initializers),*
                }
            }

        }

        #struct_vis struct #tagged_aliased_table_name<Tag: #alias_tag> {
            inner: #aliased_table_name,
            _tag: #phantom_data<fn() -> Tag>,
        }

        impl<Tag: #alias_tag> ::core::marker::Copy for #tagged_aliased_table_name<Tag> {}

        impl<Tag: #alias_tag> ::core::clone::Clone for #tagged_aliased_table_name<Tag> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<Tag: #alias_tag> ::core::default::Default for #tagged_aliased_table_name<Tag> {
            fn default() -> Self {
                Self::new()
            }
        }

        #[allow(non_upper_case_globals)]
        impl<Tag: #alias_tag> #tagged_aliased_table_name<Tag> {
            pub const fn new() -> Self {
                Self {
                    inner: #aliased_table_name::new(Tag::NAME),
                    _tag: #phantom_data,
                }
            }

            pub const fn from_inner(inner: #aliased_table_name) -> Self {
                Self {
                    inner,
                    _tag: #phantom_data,
                }
            }

            #(#tagged_const_defs)*
        }

        impl<Tag: #alias_tag> ::std::ops::Deref for #tagged_aliased_table_name<Tag> {
            type Target = #aliased_table_name;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl #taggable_alias for #aliased_table_name {
            type Tagged<Tag: #alias_tag> = #tagged_aliased_table_name<Tag>;

            fn tag<Tag: #alias_tag>(self) -> Self::Tagged<Tag> {
                #tagged_aliased_table_name::<Tag>::from_inner(self)
            }
        }


        // Implement table traits for the aliased table
        impl SQLTableInfo for #aliased_table_name {
            fn name(&self) -> &str {
                self.alias
            }

            fn schema(&self) -> ::std::option::Option<&str> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::schema(&ORIGINAL_TABLE)
            }

            fn columns(&self) -> &'static [&'static dyn SQLColumnInfo] {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::columns(&ORIGINAL_TABLE)
            }

            fn primary_key(&self) -> Option<&'static dyn SQLPrimaryKeyInfo> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::primary_key(&ORIGINAL_TABLE)
            }

            fn foreign_keys(&self) -> &'static [&'static dyn SQLForeignKeyInfo] {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::foreign_keys(&ORIGINAL_TABLE)
            }

            fn constraints(&self) -> &'static [&'static dyn SQLConstraintInfo] {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::constraints(&ORIGINAL_TABLE)
            }

            fn dependencies(&self) -> &'static [&'static dyn SQLTableInfo] {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as SQLTableInfo>::dependencies(&ORIGINAL_TABLE)
            }
        }

        // Implement PostgreSQL-specific table traits for aliased table
        impl PostgresTableInfo for #aliased_table_name {
            fn r#type(&self) -> &PostgresSchemaType {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.r#type()
            }

            fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as PostgresTableInfo>::postgres_columns(&ORIGINAL_TABLE)
            }

            fn postgres_dependencies(&self) -> &'static [&'static dyn PostgresTableInfo] {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as PostgresTableInfo>::postgres_dependencies(&ORIGINAL_TABLE)
            }
        }

        impl<'a> PostgresTable<'a> for #aliased_table_name {
            // PostgreSQL tables don't have WITHOUT_ROWID or STRICT like SQLite
        }

        // Implement core SQLTable trait for aliased table
        impl<'a> SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> for #aliased_table_name {
            type Select = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Select;
            type ForeignKeys = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::ForeignKeys;
            type PrimaryKey = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::PrimaryKey;
            type Constraints = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Constraints;
            type Insert<T> = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Insert<T>;
            type Update = <#table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Update;
            // Aliased tables alias to themselves (aliasing an already aliased table returns the same type)
            type Aliased = #aliased_table_name;

            fn alias_named(name: &'static str) -> Self::Aliased {
                #aliased_table_name::new(name)
            }
        }

        impl SQLTableMeta for #aliased_table_name {
            type ForeignKeys = <#table_name as SQLTableMeta>::ForeignKeys;
            type PrimaryKey = <#table_name as SQLTableMeta>::PrimaryKey;
            type Constraints = <#table_name as SQLTableMeta>::Constraints;
        }

        // Implement SQLSchema trait for aliased table
        impl<'a> SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>> for #aliased_table_name {
            const NAME: &'static str = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::NAME;
            const TYPE: PostgresSchemaType = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::TYPE;
            const SQL: &'static str = <#table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::SQL;
        }

        // ToSQL implementation for aliased table
        impl<'a> ToSQL<'a, PostgresValue<'a>> for #aliased_table_name {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.to_sql().alias(self.alias)
            }
        }

        impl<Tag: #alias_tag + 'static> SQLTableInfo for #tagged_aliased_table_name<Tag> {
            fn name(&self) -> &str {
                SQLTableInfo::name(&self.inner)
            }

            fn schema(&self) -> ::std::option::Option<&str> {
                SQLTableInfo::schema(&self.inner)
            }

            fn columns(&self) -> &'static [&'static dyn SQLColumnInfo] {
                SQLTableInfo::columns(&self.inner)
            }

            fn primary_key(&self) -> Option<&'static dyn SQLPrimaryKeyInfo> {
                SQLTableInfo::primary_key(&self.inner)
            }

            fn foreign_keys(&self) -> &'static [&'static dyn SQLForeignKeyInfo] {
                SQLTableInfo::foreign_keys(&self.inner)
            }

            fn constraints(&self) -> &'static [&'static dyn SQLConstraintInfo] {
                SQLTableInfo::constraints(&self.inner)
            }

            fn dependencies(&self) -> &'static [&'static dyn SQLTableInfo] {
                SQLTableInfo::dependencies(&self.inner)
            }
        }

        impl<Tag: #alias_tag + 'static> PostgresTableInfo for #tagged_aliased_table_name<Tag> {
            fn r#type(&self) -> &PostgresSchemaType {
                PostgresTableInfo::r#type(&self.inner)
            }

            fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
                PostgresTableInfo::postgres_columns(&self.inner)
            }

            fn postgres_dependencies(&self) -> &'static [&'static dyn PostgresTableInfo] {
                PostgresTableInfo::postgres_dependencies(&self.inner)
            }
        }

        impl<'a, Tag: #alias_tag + 'static> PostgresTable<'a> for #tagged_aliased_table_name<Tag> {}

        impl<'a, Tag: #alias_tag + 'static> SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> for #tagged_aliased_table_name<Tag> {
            type Select = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Select;
            type ForeignKeys = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::ForeignKeys;
            type PrimaryKey = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::PrimaryKey;
            type Constraints = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Constraints;
            type Insert<T> = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Insert<T>;
            type Update = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Update;
            type Aliased = <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased;

            fn alias_named(name: &'static str) -> Self::Aliased {
                <#aliased_table_name as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::alias_named(name)
            }
        }

        impl<Tag: #alias_tag + 'static> SQLTableMeta for #tagged_aliased_table_name<Tag> {
            type ForeignKeys = <#aliased_table_name as SQLTableMeta>::ForeignKeys;
            type PrimaryKey = <#aliased_table_name as SQLTableMeta>::PrimaryKey;
            type Constraints = <#aliased_table_name as SQLTableMeta>::Constraints;
        }

        impl<'a, Tag: #alias_tag + 'static> SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>> for #tagged_aliased_table_name<Tag> {
            const NAME: &'static str = <#aliased_table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::NAME;
            const TYPE: PostgresSchemaType = <#aliased_table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::TYPE;
            const SQL: &'static str = <#aliased_table_name as SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>>>::SQL;

            fn ddl(&self) -> SQL<'a, PostgresValue<'a>> {
                SQLSchema::ddl(&self.inner)
            }
        }

        impl<'a, Tag: #alias_tag + 'static> ToSQL<'a, PostgresValue<'a>> for #tagged_aliased_table_name<Tag> {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                ToSQL::to_sql(&self.inner)
            }
        }

        // HasSelectModel for aliased table (delegates to original)
        impl drizzle::core::HasSelectModel for #aliased_table_name {
            type SelectModel = <#table_name as drizzle::core::HasSelectModel>::SelectModel;
            const COLUMN_COUNT: usize = <#table_name as drizzle::core::HasSelectModel>::COLUMN_COUNT;
        }
        impl drizzle::core::IntoSelectTarget for #aliased_table_name {
            type Marker = drizzle::core::SelectStar;
        }

        impl<Tag: #alias_tag + 'static> drizzle::core::HasSelectModel for #tagged_aliased_table_name<Tag> {
            type SelectModel = <#aliased_table_name as drizzle::core::HasSelectModel>::SelectModel;
            const COLUMN_COUNT: usize = <#aliased_table_name as drizzle::core::HasSelectModel>::COLUMN_COUNT;
        }

        impl<Tag: #alias_tag + 'static> drizzle::core::IntoSelectTarget for #tagged_aliased_table_name<Tag> {
            type Marker = drizzle::core::SelectStar;
        }

        #struct_vis type #alias_type_name<Tag> = #tagged_aliased_table_name<Tag>;

        // Add alias() method to the original table struct
        impl #table_name {
            pub const fn alias<Tag: #alias_tag + 'static>() -> #alias_type_name<Tag> {
                #tagged_aliased_table_name::<Tag>::new()
            }
        }
    })
}
