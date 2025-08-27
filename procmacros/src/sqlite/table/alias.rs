use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use heck::ToUpperCamelCase;
use crate::sqlite::table::context::MacroContext;


/// Generates an aliased version of a table struct
/// 
/// For a table `Users` with fields `id` and `name`, this generates:
/// - `AliasedUsers` struct with `AliasedUsersId` and `AliasedUsersName` fields
/// - Each aliased field contains the table alias name
/// - `Users::alias(name: &'static str) -> AliasedUsers` method
pub fn generate_aliased_table(ctx: &MacroContext) -> syn::Result<TokenStream> {
    let table_name = &ctx.struct_ident;
    let struct_vis = &ctx.struct_vis;
    let aliased_table_name = format_ident!("Aliased{}", table_name);
    
    // Generate aliased field structs and their names
    let aliased_fields: Vec<_> = ctx.field_infos.iter().map(|field| {
        let field_name = &field.ident;
        // Use same casing as original column types to avoid conflicts
        let field_name_pascal = field_name.to_string().to_upper_camel_case();
        let aliased_field_type = format_ident!("Aliased{}{}", table_name, field_name_pascal);
        
        (field_name, aliased_field_type)
    }).collect();
    
    
    // Generate the aliased field type definitions  
    let aliased_field_definitions: Vec<TokenStream> = ctx.field_infos.iter().zip(aliased_fields.iter()).map(|(field, (_, aliased_field_type))| {
        let field_name = &field.ident;
        // Use the same naming pattern as original column types
        let field_name_pascal = field_name.to_string().to_upper_camel_case();
        let original_field_type = format_ident!("{}{}", table_name, field_name_pascal);
        
        quote! {
            #[allow(non_upper_case_globals, dead_code)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #aliased_field_type {
                alias: &'static str,
            }
            
            impl #aliased_field_type {
                pub const fn new(alias: &'static str) -> Self {
                    Self { alias }
                }
            }
            
            // Implement column info traits for the aliased field
            impl ::drizzle::core::SQLColumnInfo for #aliased_field_type {
                fn is_not_null(&self) -> bool {
                    // Forward to the original field instance with explicit trait qualification
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::is_not_null(&ORIGINAL_FIELD)
                }
                
                fn is_primary_key(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::is_primary_key(&ORIGINAL_FIELD)
                }
                
                fn is_unique(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::is_unique(&ORIGINAL_FIELD)
                }
                
                fn name(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::name(&ORIGINAL_FIELD)
                }
                
                fn r#type(&self) -> &str {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::r#type(&ORIGINAL_FIELD)
                }
                
                fn has_default(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::core::SQLColumnInfo>::has_default(&ORIGINAL_FIELD)
                }
                
                fn table(&self) -> &dyn ::drizzle::core::SQLTableInfo {
                    // This is tricky - we need a static reference but each column has different alias
                    // For now, return the original table info
                    static ORIGINAL_TABLE: #table_name = #table_name::new();
                    &ORIGINAL_TABLE
                }
            }
            
            // Implement SQLite-specific column info traits
            impl ::drizzle::sqlite::traits::SQLiteColumnInfo for #aliased_field_type {
                fn is_autoincrement(&self) -> bool {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::sqlite::traits::SQLiteColumnInfo>::is_autoincrement(&ORIGINAL_FIELD)
                }
                
                fn table(&self) -> &dyn ::drizzle::sqlite::traits::SQLiteTableInfo {
                    // This is tricky - we need a static reference to our aliased table info
                    // For now, we'll use a workaround
                    todo!("Need to implement aliased SQLiteTableInfo reference")
                }
                
                fn foreign_key(&self) -> Option<&'static dyn ::drizzle::sqlite::traits::SQLiteColumnInfo> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    <#original_field_type as ::drizzle::sqlite::traits::SQLiteColumnInfo>::foreign_key(&ORIGINAL_FIELD)
                }
            }
            
            // AsColumnInfo is already implemented via blanket impl for SQLiteColumnInfo
            
            // Implement SQLColumn trait for aliased field
            impl<'a> ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #aliased_field_type {
                type Table = #aliased_table_name;
                type TableType = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::TableType;
                type Type = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::Type;
                
                const PRIMARY_KEY: bool = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::PRIMARY_KEY;
                const NOT_NULL: bool = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::NOT_NULL;
                const UNIQUE: bool = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::UNIQUE;
                const DEFAULT: Option<Self::Type> = <#original_field_type as ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>>>::DEFAULT;
                
                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    static ORIGINAL_FIELD: #original_field_type = #original_field_type::new();
                    ORIGINAL_FIELD.default_fn()
                }
            }
            
            // Implement SQLiteColumn trait for aliased field
            impl<'a> ::drizzle::sqlite::traits::SQLiteColumn<'a> for #aliased_field_type {
                const AUTOINCREMENT: bool = <#original_field_type as ::drizzle::sqlite::traits::SQLiteColumn<'a>>::AUTOINCREMENT;
            }
            
            // Implement SQLSchema trait for aliased field
            impl<'a> ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>> for #aliased_field_type {
                const NAME: &'a str = <#original_field_type as ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>>>::NAME;
                const TYPE: &'a str = <#original_field_type as ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>>>::TYPE;
                
                const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = <#original_field_type as ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a>>>::SQL;
            }
            
            // ToSQL implementation that uses the alias
            impl<'a, V: ::drizzle::core::SQLParam + 'a> ::drizzle::core::ToSQL<'a, V> for #aliased_field_type {
                fn to_sql(&self) -> ::drizzle::core::SQL<'a, V> {
                    use ::drizzle::core::SQLColumnInfo;
                    ::drizzle::core::SQL::raw(format!(r#""{}"."{}""#, self.alias, self.name()))
                }
            }
        }
    }).collect();
    
    // Generate the aliased table struct fields
    let aliased_struct_fields: Vec<TokenStream> = aliased_fields.iter().map(|(field_name, aliased_type)| {
        quote! {
            #struct_vis #field_name: #aliased_type
        }
    }).collect();
    
    // Generate field initializers for the alias() method
    let field_initializers: Vec<TokenStream> = aliased_fields.iter().map(|(field_name, aliased_type)| {
        quote! {
            #field_name: #aliased_type::new(alias)
        }
    }).collect();
    
    Ok(quote! {
        
        // Generate all aliased field type definitions
        #(#aliased_field_definitions)*
        
        // Generate the aliased table struct
        #[allow(non_upper_case_globals, dead_code)]
        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
        
        
        // Implement table traits for the aliased table
        impl ::drizzle::core::SQLTableInfo for #aliased_table_name {
            fn name(&self) -> &str {
                self.alias
            }
            
            fn columns(&self) -> Box<[&'static dyn ::drizzle::core::SQLColumnInfo]> {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as ::drizzle::core::SQLTableInfo>::columns(&ORIGINAL_TABLE)
            }
        }
        
        // Implement SQLite-specific table traits for aliased table
        impl ::drizzle::sqlite::traits::SQLiteTableInfo for #aliased_table_name {
            fn r#type(&self) -> &::drizzle::sqlite::common::SQLiteSchemaType {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.r#type()
            }
            
            fn without_rowid(&self) -> bool {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.without_rowid()
            }
            
            fn strict(&self) -> bool {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.strict()
            }
            
            fn columns(&self) -> Box<[&'static dyn ::drizzle::sqlite::traits::SQLiteColumnInfo]> {
                // TODO: This is tricky because we need static references but each alias instance
                // has a different alias string. For now, return original columns.
                // The individual aliased fields can still be accessed directly via table.field
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                <#table_name as ::drizzle::sqlite::traits::SQLiteTableInfo>::columns(&ORIGINAL_TABLE)
            }
        }
        
        impl<'a> ::drizzle::sqlite::traits::SQLiteTable<'a> for #aliased_table_name {
            const WITHOUT_ROWID: bool = <#table_name as ::drizzle::sqlite::traits::SQLiteTable<'a>>::WITHOUT_ROWID;
            const STRICT: bool = <#table_name as ::drizzle::sqlite::traits::SQLiteTable<'a>>::STRICT;
        }
        
        // Implement core SQLTable trait for aliased table
        impl<'a> ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #aliased_table_name {
            type Select = <#table_name as ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::Select;
            type Insert<T> = <#table_name as ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::Insert<T>;
            type Update = <#table_name as ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::Update;
        }
        
        // Implement SQLSchema trait for aliased table
        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #aliased_table_name {
            const NAME: &'a str = <#table_name as ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::NAME;
            const TYPE: ::drizzle::sqlite::common::SQLiteSchemaType = <#table_name as ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::TYPE;
            const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = <#table_name as ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>>>::SQL;
        }
        
        // ToSQL implementation for aliased table
        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #aliased_table_name {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                static ORIGINAL_TABLE: #table_name = #table_name::new();
                ORIGINAL_TABLE.to_sql().alias(self.alias)
            }
        }
        
        // Add alias() method to the original table struct
        impl #table_name {
            pub const fn alias(alias: &'static str) -> #aliased_table_name {
                #aliased_table_name::new(alias)
            }
        }
    })
}
