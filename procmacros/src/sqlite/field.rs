use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{
    Attribute, Error, Expr, ExprPath, Field, Ident, Lit, LitStr, Meta, Result, Token, Type,
    parse::{Parse, ParseStream},
};

/// Enum representing supported SQLite column types
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum SQLiteType {
    Integer,
    Text,
    Blob,
    Real,
    Numeric,
    #[default]
    Any,
}

impl Display for SQLiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SQLiteType::Integer => "INTEGER",
            SQLiteType::Text => "TEXT",
            SQLiteType::Blob => "BLOB",
            SQLiteType::Real => "REAL",
            SQLiteType::Numeric => "NUMERIC",
            SQLiteType::Any => "ANY",
        };
        f.write_fmt(format_args!("{value}"))
    }
}

impl SQLiteType {
    /// Convert from attribute name to enum variant
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
        match name {
            "integer" => Some(SQLiteType::Integer),
            "text" => Some(SQLiteType::Text),
            "blob" => Some(SQLiteType::Blob),
            "real" => Some(SQLiteType::Real),
            "number" | "numeric" => Some(SQLiteType::Numeric),
            "any" => Some(Default::default()),
            _ => None,
        }
    }

    /// Get the SQL type string for this type
    pub(crate) fn to_sql_type(&self) -> &'static str {
        match self {
            SQLiteType::Integer => "INTEGER",
            SQLiteType::Text => "TEXT",
            SQLiteType::Blob => "BLOB",
            SQLiteType::Real => "REAL",
            SQLiteType::Numeric => "NUMERIC",
            SQLiteType::Any => "ANY",
        }
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        match (self, flag) {
            (SQLiteType::Integer, "autoincrement") => true,
            (SQLiteType::Text | SQLiteType::Blob, "json") => true,
            (SQLiteType::Text | SQLiteType::Integer, "enum") => true,
            (_, "primary") | (_, "primary_key") | (_, "unique") => true,
            _ => false,
        }
    }

    /// Validate a flag for this column type, returning an error with SQLite docs link if invalid
    pub(crate) fn validate_flag(&self, flag: &str, attr: &Attribute) -> Result<()> {
        if !self.is_valid_flag(flag) {
            let error_msg = match flag {
                "autoincrement" => {
                    "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY columns.\n\
                     See: https://sqlite.org/autoinc.html\n\
                     Use: #[integer(primary_key, autoincrement)]"
                }
                "json" => {
                    "JSON serialization is only supported for TEXT or BLOB column types.\n\
                     See: https://sqlite.org/json1.html\n\
                     Use: #[text(json)] or #[blob(json)]"
                }
                "enum" => {
                    "Enum serialization is supported for TEXT (string) or INTEGER (discriminant) columns.\n\
                     Use: #[text(enum)] or #[integer(enum)]"
                }
                "not_null" => {
                    "Use Option<T> in your struct field to represent nullable columns instead of 'not_null' attribute.\n\
                     See: https://sqlite.org/lang_createtable.html#notnullconst\n\
                     Example: pub field: Option<String> for nullable TEXT"
                }
                _ => return Ok(()),
            };

            return Err(Error::new_spanned(attr, error_msg));
        }

        Ok(())
    }
}

/// Comprehensive field information for code generation
#[derive(Clone)]
pub(crate) struct FieldInfo<'a> {
    // Basic field identifiers and types
    pub(crate) ident: &'a Ident,
    pub(crate) field_type: &'a Type,
    pub(crate) base_type: &'a Type,

    // Database mapping
    pub(crate) column_name: String,
    pub(crate) sql_definition: String,

    // Field properties
    pub(crate) is_nullable: bool,
    pub(crate) has_default: bool,
    pub(crate) is_primary: bool,
    pub(crate) is_autoincrement: bool,
    pub(crate) is_unique: bool,
    pub(crate) is_json: bool,
    pub(crate) is_enum: bool,
    pub(crate) is_uuid: bool,
    pub(crate) column_type: SQLiteType,

    // Attribute values
    pub(crate) default_value: Option<Expr>,
    pub(crate) default_fn: Option<Expr>,
    pub(crate) references_path: Option<ExprPath>,
    pub(crate) name: Option<String>,

    // Type representations for models
    pub(crate) select_type: Option<TokenStream>,
    pub(crate) insert_type: Option<TokenStream>,
    pub(crate) update_type: Option<TokenStream>,
}

impl<'a> Parse for FieldInfo<'a> {
    fn parse(input: ParseStream) -> Result<Self> {
        // This implementation is a placeholder since FieldInfo requires
        // references to fields that can't be obtained solely from parsing.
        // The actual parsing happens in parse_attribute_args and from_field.
        Err(Error::new(
            input.span(),
            "FieldInfo cannot be directly parsed from a token stream",
        ))
    }
}

impl<'a> FieldInfo<'a> {
    /// Parse attribute arguments for field attributes
    pub(crate) fn parse_attribute_args(
        input: ParseStream,
    ) -> Result<(
        Option<Expr>,
        Option<Expr>,
        Option<Expr>,
        Option<Expr>,
        HashSet<String>,
    )> {
        let mut default_value = None;
        let mut default_fn = None;
        let mut references = None;
        let mut name = None;
        let mut flags = HashSet::new();

        // If the input is empty, return empty collections
        if input.is_empty() {
            return Ok((default_value, default_fn, references, name, flags));
        }

        // Parse a comma-separated list of expressions
        let punctuated = input.parse_terminated(Expr::parse, Token![,])?;

        for expr in punctuated {
            match expr {
                Expr::Path(path_expr) => {
                    // Handle flags like primary_key, not_null, etc.
                    if let Some(flag_ident) = path_expr.path.get_ident() {
                        let flag_str = flag_ident.to_string();
                        if flag_str == "default" {
                            // Handle bare 'default' keyword - use Default::default()
                            let default_path: syn::ExprPath = syn::parse_quote!(Default::default);
                            default_fn = Some(syn::Expr::Path(default_path));
                        } else {
                            flags.insert(flag_str);
                        }
                    }
                }
                Expr::Assign(assign_expr) => {
                    // Handle named parameters (e.g., default = "value")
                    if let Expr::Path(path_expr) = &*assign_expr.left {
                        if let Some(param_ident) = path_expr.path.get_ident() {
                            let param_name = param_ident.to_string();
                            match param_name.as_str() {
                                "default" => default_value = Some(*assign_expr.right.clone()),
                                "default_fn" => default_fn = Some(*assign_expr.right.clone()),
                                "references" => references = Some(*assign_expr.right.clone()),
                                "name" => name = Some(*assign_expr.right.clone()),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((default_value, default_fn, references, name, flags))
    }

    /// Parse field information from a Field
    pub(crate) fn from_field(field: &'a Field, is_part_of_composite_pk: bool) -> Result<Self> {
        let field_name = field.ident.as_ref().ok_or_else(|| {
            Error::new_spanned(
                field,
                "All struct fields must have names. Tuple structs are not supported.\n\
                 Example: pub field_name: String (not pub String)",
            )
        })?;
        let field_type = &field.ty;

        // Initialize collections for parsed attributes
        let mut flags = HashSet::new();
        let mut column_type = Default::default();
        let mut default_value = None;
        let mut default_fn = None;
        let mut references_path = None;
        let mut attr_name = None;

        // Parse attributes
        for attr in &field.attrs {
            // Check if the attribute path is one of our supported column types
            let path_ident = attr.path().get_ident();
            if let Some(ident) = path_ident {
                let type_name = ident.to_string();

                // Check if this is a column type attribute
                if let Some(sqlite_type) = SQLiteType::from_attribute_name(&type_name) {
                    // Set the column type
                    column_type = sqlite_type.clone();

                    // Handle the case of an empty attribute (e.g., #[text])
                    if let Meta::Path(_) = attr.meta {
                        // This is an attribute without arguments, like #[text]
                        continue;
                    }

                    // Parse the arguments using our custom parser
                    if let Ok((
                        default_val,
                        default_fn_val,
                        references_val,
                        name_val,
                        parsed_flags,
                    )) = attr.parse_args_with(Self::parse_attribute_args)
                    {
                        // Validate attributes based on column type
                        for flag in &parsed_flags {
                            // Validate the flag for this column type
                            sqlite_type.validate_flag(flag, attr)?;
                        }

                        // Set values
                        if let Some(val) = default_val {
                            default_value = Some(val);
                        }

                        if let Some(val) = default_fn_val {
                            default_fn = Some(val);
                        }

                        if let Some(val) = references_val {
                            if let Expr::Path(path) = val {
                                references_path = Some(path);
                            }
                        }

                        // Extract name from attribute if present using modern pattern matching
                        if let Some(Expr::Lit(expr_lit)) = name_val
                            && let Lit::Str(lit_str) = expr_lit.lit
                        {
                            attr_name = Some(lit_str.value());
                        }

                        // Merge the flags
                        flags.extend(parsed_flags);
                    }
                }
            }
        }

        // Get column name (attribute name or field name)
        let column_name = attr_name.clone().unwrap_or_else(|| field_name.to_string());

        // Parse flags and options
        let is_primary = flags.contains("primary_key") || flags.contains("primary");
        let is_autoincrement = flags.contains("autoincrement");
        let is_unique = flags.contains("unique");
        let is_nullable = is_option_type(field_type);
        let is_json = flags.contains("json");
        let is_enum = flags.contains("enum");
        let has_default = default_value.is_some() || default_fn.is_some();

        // Determine base type (T from Option<T> or T)
        let base_type: &Type = if is_nullable {
            get_option_inner_type(field_type).unwrap_or(field_type)
        } else {
            field_type
        };

        // Create column definition
        let mut sql = format!("{} {}", column_name, column_type.to_sql_type());

        // Add generic column constraints
        if is_primary && !is_part_of_composite_pk {
            sql.push_str(" PRIMARY KEY");
        }

        if !is_nullable {
            sql.push_str(" NOT NULL");
        }

        if is_unique {
            sql.push_str(" UNIQUE");
        }

        // Add default value using modern pattern matching
        if let Some(Expr::Lit(expr_lit)) = &default_value {
            let default_sql = match &expr_lit.lit {
                Lit::Int(i) => format!(" DEFAULT {}", i),
                Lit::Float(f) => format!(" DEFAULT {}", f),
                Lit::Bool(b) => format!(" DEFAULT {}", b.value() as i64),
                Lit::Str(s) => format!(" DEFAULT '{}'", s.value()),
                _ => String::new(),
            };
            sql.push_str(&default_sql);
        }

        // Create type representations for models
        let select_type = if !is_nullable || has_default {
            Some(quote!(#base_type))
        } else {
            Some(quote!(::std::option::Option<#base_type>))
        };

        let insert_type = if is_nullable || has_default || is_primary {
            Some(quote!(::std::option::Option<#base_type>))
        } else {
            Some(quote!(#base_type))
        };

        let update_type = Some(quote!(::std::option::Option<#base_type>));
        let is_uuid = base_type.to_token_stream().to_string().eq("Uuid");

        // Add helpful warnings for common mistakes
        if is_autoincrement && !matches!(column_type, SQLiteType::Integer) {
            return Err(Error::new_spanned(
                field,
                "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY.\n\
                 See: https://sqlite.org/autoinc.html\n\
                 Hint: Change column type to '#[integer(primary, autoincrement)]'",
            ));
        }

        if is_autoincrement && !is_primary {
            return Err(Error::new_spanned(
                field,
                "AUTOINCREMENT requires PRIMARY KEY constraint.\n\
                 See: https://sqlite.org/autoinc.html\n\
                 Hint: Add 'primary' flag: '#[integer(primary, autoincrement)]'",
            ));
        }

        if default_value.is_some() && default_fn.is_some() {
            return Err(Error::new_spanned(
                field,
                "Cannot specify both 'default' (compile-time literal) and 'default_fn' (runtime function).\n\
                 Choose one: either 'default = literal' or 'default_fn = function'\n\
                 Examples:\n  #[text(default = \"hello\")] for compile-time defaults\n  #[text(default_fn = String::new)] for runtime defaults",
            ));
        }

        // Validate UUID fields can only use BLOB column type
        if is_uuid && !matches!(column_type, SQLiteType::Blob) {
            return Err(Error::new_spanned(
                field,
                "UUID fields must use BLOB column type for optimal performance and compatibility.\n\
                 UUIDs are stored as 16-byte binary data in SQLite.\n\
                 See: https://sqlite.org/datatype3.html#storage_classes_and_datatypes\n\
                 Use: #[blob] instead of #[text] for UUID fields\n\
                 Example: #[blob(primary, default_fn = uuid::Uuid::new_v4)] pub id: uuid::Uuid",
            ));
        }

        // Create the FieldInfo struct
        Ok(FieldInfo {
            ident: field_name,
            field_type,
            base_type,
            column_name,
            sql_definition: sql,
            is_nullable,
            has_default,
            is_primary,
            is_autoincrement,
            is_unique,
            is_json,
            is_enum,
            is_uuid,
            column_type,
            default_value,
            default_fn,
            references_path,
            name: attr_name,
            select_type,
            insert_type,
            update_type,
        })
    }

    /// Get the model field type for this field in the SelectModel
    pub(crate) fn get_select_type(&self) -> TokenStream {
        self.select_type.clone().unwrap_or_else(|| {
            let base_type = self.base_type;
            if !self.is_nullable || self.has_default {
                quote!(#base_type)
            } else {
                quote!(::std::option::Option<#base_type>)
            }
        })
    }

    /// Get the model field type for this field in the InsertModel
    pub(crate) fn get_insert_type(&self) -> TokenStream {
        self.insert_type.clone().unwrap_or_else(|| {
            let base_type = self.base_type;
            // All primary keys should be optional in insert models to avoid unique constraint conflicts
            // when default values (like 0) are used
            if self.is_nullable || self.has_default || self.is_primary {
                quote!(::std::option::Option<#base_type>)
            } else {
                quote!(#base_type)
            }
        })
    }

    /// Get the model field type for this field in the UpdateModel
    pub(crate) fn get_update_type(&self) -> TokenStream {
        self.update_type.clone().unwrap_or_else(|| {
            let base_type = self.base_type;
            quote!(::std::option::Option<#base_type>)
        })
    }
}

#[derive(Clone)]
pub(crate) enum Relationship {
    One(LitStr),
    Many(LitStr),
}

// Helper function to check if a type is an Option<T>
pub(crate) fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                return true;
            }
        }
    }
    false
}

// Helper function to get the inner type of Option<T>
pub(crate) fn get_option_inner_type<'a>(ty: &'a syn::Type) -> Option<&'a syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
    {
        return Some(inner_type);
    }
    None
}
