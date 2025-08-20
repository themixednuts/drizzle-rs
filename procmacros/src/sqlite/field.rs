use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::{collections::HashSet, fmt::Display};
use syn::{
    Attribute, Error, Expr, ExprPath, Field, Ident, Lit, Meta, Result, Token, Type,
    parse::ParseStream,
};

/// Enum representing supported SQLite column types.
///
/// These correspond to the [SQLite storage classes](https://sqlite.org/datatype3.html#storage_classes_and_datatypes).
/// Each type maps to specific Rust types and has different capabilities for constraints and features.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) enum SQLiteType {
    /// SQLite INTEGER type - stores signed integers up to 8 bytes.
    ///
    /// See: <https://sqlite.org/datatype3.html#integer_datatype>
    ///
    /// Supports: primary keys, autoincrement, enums (discriminant storage)
    Integer,

    /// SQLite TEXT type - stores text in UTF-8, UTF-16BE, or UTF-16LE encoding.
    ///
    /// See: <https://sqlite.org/datatype3.html#text_datatype>
    ///
    /// Supports: enums (variant name storage), JSON serialization
    Text,

    /// SQLite BLOB type - stores binary data exactly as input.
    ///
    /// See: <https://sqlite.org/datatype3.html#blob_datatype>
    ///
    /// Supports: JSON serialization, UUID storage
    Blob,

    /// SQLite REAL type - stores floating point values as 8-byte IEEE floating point numbers.
    ///
    /// See: <https://sqlite.org/datatype3.html#real_datatype>
    Real,

    /// SQLite NUMERIC type - stores values as INTEGER, REAL, or TEXT depending on the value.
    ///
    /// See: <https://sqlite.org/datatype3.html#numeric_datatype>
    Numeric,

    /// SQLite ANY type - no type affinity, can store any type of data.
    ///
    /// See: <https://sqlite.org/datatype3.html#type_affinity>
    #[default]
    Any,
}

impl Display for SQLiteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sql_type())
    }
}

impl SQLiteType {
    /// Convert from attribute name to enum variant
    pub(crate) fn from_attribute_name(name: &str) -> Option<Self> {
        match name {
            "integer" => Some(Self::Integer),
            "text" => Some(Self::Text),
            "blob" => Some(Self::Blob),
            "real" => Some(Self::Real),
            "number" | "numeric" => Some(Self::Numeric),
            "boolean" => Some(Self::Integer), // Store booleans as integers (0/1)
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    /// Get the SQL type string for this type
    pub(crate) fn to_sql_type(&self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Text => "TEXT",
            Self::Blob => "BLOB",
            Self::Real => "REAL",
            Self::Numeric => "NUMERIC",
            Self::Any => "ANY",
        }
    }

    /// Check if a flag is valid for this column type
    pub(crate) fn is_valid_flag(&self, flag: &str) -> bool {
        matches!(
            (self, flag),
            (Self::Integer, "autoincrement")
                | (Self::Text | Self::Blob, "json")
                | (Self::Text | Self::Integer, "enum")
                | (_, "primary" | "primary_key" | "unique")
        )
    }

    /// Validate a flag for this column type, returning an error with SQLite docs link if invalid.
    ///
    /// Provides helpful error messages with links to relevant SQLite documentation
    /// when incompatible flag/type combinations are used.
    pub(crate) fn validate_flag(&self, flag: &str, attr: &Attribute) -> Result<()> {
        if !self.is_valid_flag(flag) {
            let error_msg = match flag {
                "autoincrement" => {
                    "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY columns.\n\
                     \n\
                     SQLite AUTOINCREMENT ensures that new rows get unique rowids, but it only \
                     works on INTEGER PRIMARY KEY columns in regular (non-WITHOUT ROWID) tables.\n\
                     \n\
                     See: https://sqlite.org/autoinc.html\n\
                     Use: #[integer(primary, autoincrement)]"
                }
                "json" => {
                    "JSON serialization is only supported for TEXT or BLOB column types.\n\
                     \n\
                     JSON data can be stored as TEXT (human-readable) or BLOB (binary). \
                     The choice affects storage size and query capabilities.\n\
                     \n\
                     See: https://sqlite.org/json1.html\n\
                     Use: #[text(json)] or #[blob(json)]"
                }
                "enum" => {
                    "Enum serialization is supported for TEXT (string) or INTEGER (discriminant) columns.\n\
                     \n\
                     - TEXT storage: stores variant names like 'Active', 'Inactive'\n\
                     - INTEGER storage: stores discriminant values like 0, 1, 2\n\
                     \n\
                     Use: #[text(enum)] or #[integer(enum)]"
                }
                "not_null" => {
                    "Use Option<T> in your struct field to represent nullable columns instead of 'not_null' attribute.\n\
                     \n\
                     Drizzle RS uses Rust's type system for nullability:\n\
                     - Field type `T` = NOT NULL column\n\
                     - Field type `Option<T>` = NULL allowed column\n\
                     \n\
                     See: https://sqlite.org/lang_createtable.html#notnullconst\n\
                     Example: pub email: Option<String> for nullable TEXT"
                }
                _ => return Ok(()),
            };

            return Err(Error::new_spanned(attr, error_msg));
        }

        Ok(())
    }
}

/// Foreign key reference information
#[derive(Debug, Clone)]
pub(crate) struct ForeignKeyReference {
    /// The referenced table identifier (e.g., "User" from User::id)
    pub(crate) table_ident: Ident,
    /// The referenced column identifier (e.g., "id" from User::id)
    pub(crate) column_ident: Ident,
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

    // Foreign key support
    pub(crate) foreign_key: Option<ForeignKeyReference>,

    // Attribute values
    pub(crate) default_value: Option<Expr>,
    pub(crate) default_fn: Option<Expr>,

    // Type representations for models
    pub(crate) select_type: Option<TokenStream>,
    pub(crate) update_type: Option<TokenStream>,
}

/// Parse attribute items, handling reserved keywords like 'enum'
fn parse_item(input: ParseStream) -> Result<Expr> {
    let lookahead = input.lookahead1();

    if lookahead.peek(Token![enum]) {
        input.parse::<Token![enum]>()?;
        let ident = syn::Ident::new("enum", proc_macro2::Span::call_site());
        Ok(syn::Expr::Path(syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: syn::Path::from(ident),
        }))
    } else {
        input.parse::<Expr>()
    }
}

#[derive(Default)]
struct ParsedArgs {
    default_value: Option<Expr>,
    default_fn: Option<Expr>,
    references: Option<Expr>,
    name: Option<Expr>,
    flags: HashSet<String>,
}

#[derive(Default)]
struct AttributeData {
    column_type: SQLiteType,
    flags: HashSet<String>,
    default_value: Option<Expr>,
    default_fn: Option<Expr>,
    references_path: Option<ExprPath>,
    attr_name: Option<String>,
}

struct FieldProperties {
    is_primary: bool,
    is_autoincrement: bool,
    is_unique: bool,
    is_json: bool,
    is_enum: bool,
    is_uuid: bool,
    has_default: bool,
}

impl FieldProperties {
    fn from_flags_and_types(flags: &HashSet<String>, _field_type: &Type, base_type: &Type) -> Self {
        Self {
            is_primary: flags.contains("primary_key") || flags.contains("primary"),
            is_autoincrement: flags.contains("autoincrement"),
            is_unique: flags.contains("unique"),
            is_json: flags.contains("json"),
            is_enum: flags.contains("enum"),
            is_uuid: base_type.to_token_stream().to_string().contains("Uuid"),
            has_default: false, // Will be set in build() based on actual values
        }
    }
}

impl<'a> FieldInfo<'a> {
    /// Parse attribute arguments, extracting flags and named parameters
    fn parse_args(input: ParseStream) -> Result<ParsedArgs> {
        if input.is_empty() {
            return Ok(ParsedArgs::default());
        }

        let items = input.parse_terminated(parse_item, Token![,])?;
        let mut args = ParsedArgs::default();

        items.into_iter().for_each(|expr| match expr {
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    match ident.to_string().as_str() {
                        "default" => args.default_fn = Some(syn::parse_quote!(Default::default)),
                        flag => {
                            args.flags.insert(flag.to_string());
                        }
                    }
                }
            }
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left
                    && let Some(param) = path.path.get_ident()
                {
                    match param.to_string().as_str() {
                        "default" => args.default_value = Some(*assign.right),
                        "default_fn" => args.default_fn = Some(*assign.right),
                        "references" => args.references = Some(*assign.right),
                        "name" => args.name = Some(*assign.right),
                        _ => {}
                    }
                }
            }
            _ => {}
        });

        Ok(args)
    }

    /// Parse field information from a struct field
    pub(crate) fn from_field(field: &'a Field, is_part_of_composite_pk: bool) -> Result<Self> {
        let Some(field_name) = &field.ident else {
            return Err(Error::new_spanned(
                field,
                "All struct fields must have names. Tuple structs are not supported.\n\
                 Example: pub field_name: String",
            ));
        };

        let attrs = Self::parse_attributes(&field.attrs)?;
        Self::build(field_name, &field.ty, attrs, is_part_of_composite_pk)
    }

    /// Parse field attributes to extract column information
    fn parse_attributes(attrs: &[Attribute]) -> Result<AttributeData> {
        attrs
            .iter()
            .filter_map(|attr| {
                let ident = attr.path().get_ident()?;
                let type_name = ident.to_string();
                let column_type = SQLiteType::from_attribute_name(&type_name)?;
                Some((attr, column_type))
            })
            .try_fold(AttributeData::default(), |mut data, (attr, column_type)| {
                data.column_type = column_type.clone();

                // Handle empty attributes like #[text]
                if matches!(&attr.meta, Meta::Path(_)) {
                    return Ok(data);
                }

                let Ok(args) = attr.parse_args_with(Self::parse_args) else {
                    return Ok(data); // Skip unparseable attributes
                };

                // Validate flags
                args.flags
                    .iter()
                    .try_for_each(|flag| column_type.validate_flag(flag, attr))?;

                // Extract values
                data.flags.extend(args.flags);
                data.default_value = data.default_value.or(args.default_value);
                data.default_fn = data.default_fn.or(args.default_fn);

                if let Some(Expr::Path(path)) = args.references {
                    data.references_path = Some(path);
                }

                if let Some(Expr::Lit(expr_lit)) = args.name
                    && let Lit::Str(lit_str) = expr_lit.lit
                {
                    data.attr_name = Some(lit_str.value());
                }

                Ok(data)
            })
    }

    /// Build FieldInfo from parsed components
    fn build(
        field_name: &'a Ident,
        field_type: &'a Type,
        attrs: AttributeData,
        is_part_of_composite_pk: bool,
    ) -> Result<Self> {
        let column_name = attrs
            .attr_name
            .clone()
            .unwrap_or_else(|| field_name.to_string());
        let is_nullable = is_option_type(field_type);
        let base_type = if is_nullable {
            extract_option_inner(field_type).unwrap_or(field_type)
        } else {
            field_type
        };

        let mut properties =
            FieldProperties::from_flags_and_types(&attrs.flags, field_type, base_type);
        properties.has_default = attrs.default_value.is_some() || attrs.default_fn.is_some();

        Self::validate_constraints(
            &attrs.column_type,
            &properties,
            &attrs.default_value,
            &attrs.default_fn,
            field_name,
        )?;

        let sql_definition = build_sql_definition(
            &column_name,
            &attrs.column_type,
            properties.is_primary && !is_part_of_composite_pk,
            !is_nullable,
            properties.is_unique,
            properties.is_autoincrement,
            &attrs.default_value,
        );

        // Detect foreign key reference from the attributes (references = Table::column)
        let foreign_key = attrs
            .references_path
            .as_ref()
            .and_then(detect_foreign_key_reference_from_path);

        Ok(FieldInfo {
            ident: field_name,
            field_type,
            base_type,
            column_name,
            sql_definition,
            is_nullable,
            has_default: properties.has_default,
            is_primary: properties.is_primary,
            is_autoincrement: properties.is_autoincrement,
            is_unique: properties.is_unique,
            is_json: properties.is_json,
            is_enum: properties.is_enum,
            is_uuid: properties.is_uuid,
            column_type: attrs.column_type,
            foreign_key,
            default_value: attrs.default_value,
            default_fn: attrs.default_fn,
            select_type: Some(select_type(base_type, is_nullable, properties.has_default)),
            update_type: Some(update_type(base_type)),
        })
    }

    /// Validate field constraints and configuration
    fn validate_constraints(
        column_type: &SQLiteType,
        props: &FieldProperties,
        default_value: &Option<Expr>,
        default_fn: &Option<Expr>,
        field_name: &Ident,
    ) -> Result<()> {
        let validations = [
            (
                props.is_autoincrement && !matches!(column_type, SQLiteType::Integer),
                "AUTOINCREMENT can only be used with INTEGER PRIMARY KEY.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Change column type to '#[integer(primary, autoincrement)]'",
            ),
            (
                props.is_autoincrement && !props.is_primary,
                "AUTOINCREMENT requires PRIMARY KEY constraint.\n\
              See: https://sqlite.org/autoinc.html\n\
              Hint: Add 'primary' flag: '#[integer(primary, autoincrement)]'",
            ),
            (
                default_value.is_some() && default_fn.is_some(),
                "Cannot specify both 'default' (compile-time literal) and 'default_fn' (runtime function).\n\
              Choose one: either 'default = literal' or 'default_fn = function'\n\
              Examples:\n  #[text(default = \"hello\")] for compile-time defaults\n  #[text(default_fn = String::new)] for runtime defaults",
            ),
            (
                props.is_uuid && !matches!(column_type, SQLiteType::Blob | SQLiteType::Text),
                "UUID fields must use either BLOB or TEXT column type.\n\
              BLOB storage: Efficient 16-byte binary format (recommended)\n\
              TEXT storage: Human-readable string format\n\
              See: https://sqlite.org/datatype3.html#storage_classes_and_datatypes\n\
              Examples:\n  #[blob(primary, default_fn = uuid::Uuid::new_v4)] pub id: uuid::Uuid\n  #[text(default_fn = uuid::Uuid::new_v4)] pub uuid_text: uuid::Uuid",
            ),
        ];

        validations
            .iter()
            .find(|(condition, _)| *condition)
            .map_or(Ok(()), |(_, msg)| Err(Error::new_spanned(field_name, msg)))
    }
}

/// Build SQL column definition string
fn build_sql_definition(
    column_name: &str,
    column_type: &SQLiteType,
    is_primary_single: bool,
    is_not_null: bool,
    is_unique: bool,
    is_autoincrement: bool,
    default_value: &Option<Expr>,
) -> String {
    let mut sql = format!("{} {}", column_name, column_type.to_sql_type());

    // Handle primary key with potential autoincrement
    if is_primary_single {
        sql.push_str(" PRIMARY KEY");
        if is_autoincrement {
            sql.push_str(" AUTOINCREMENT");
        }
    }

    // Add NOT NULL constraint
    if is_not_null {
        sql.push_str(" NOT NULL");
    }

    // Add UNIQUE constraint
    if is_unique {
        sql.push_str(" UNIQUE");
    }

    if let Some(Expr::Lit(expr_lit)) = default_value {
        let default_val = match &expr_lit.lit {
            Lit::Int(i) => format!(" DEFAULT {i}"),
            Lit::Float(f) => format!(" DEFAULT {f}"),
            Lit::Bool(b) => format!(" DEFAULT {}", b.value() as i64),
            Lit::Str(s) => format!(" DEFAULT \"{}\"", s.value()),
            _ => String::new(),
        };
        sql.push_str(&default_val);
    }

    sql
}

/// Generate the appropriate type for select models
fn select_type(base_type: &Type, is_nullable: bool, has_default: bool) -> TokenStream {
    if !is_nullable || has_default {
        quote!(#base_type)
    } else {
        quote!(::std::option::Option<#base_type>)
    }
}

/// Generate the appropriate type for update models
fn update_type(base_type: &Type) -> TokenStream {
    quote!(::std::option::Option<#base_type>)
}

impl<'a> FieldInfo<'a> {
    /// Get the model field type for this field in the SelectModel
    pub(crate) fn get_select_type(&self) -> TokenStream {
        self.select_type
            .clone()
            .unwrap_or_else(|| select_type(self.base_type, self.is_nullable, self.has_default))
    }

    /// Get the model field type for this field in the UpdateModel
    pub(crate) fn get_update_type(&self) -> TokenStream {
        self.update_type
            .clone()
            .unwrap_or_else(|| update_type(self.base_type))
    }
}

/// Check if a type is an Option<T>
pub(crate) fn is_option_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path)
        if type_path.path.segments.last()
            .is_some_and(|seg| seg.ident == "Option"))
}

/// Extract the inner type from Option<T>
pub(crate) fn extract_option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;

    if segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
    {
        Some(inner_type)
    } else {
        None
    }
}

/// Detect if an ExprPath is a foreign key reference (Table::column syntax)
/// Returns (table_ident, column_ident) if the path matches the pattern
pub(crate) fn detect_foreign_key_reference_from_path(
    path: &ExprPath,
) -> Option<ForeignKeyReference> {
    // Check if this is a path with exactly 2 segments (Table::column)
    if path.path.segments.len() == 2 {
        let table_ident = path.path.segments.first()?.ident.clone();
        let column_ident = path.path.segments.last()?.ident.clone();

        // Basic validation: ensure both segments exist and are valid identifiers
        if !table_ident.to_string().is_empty() && !column_ident.to_string().is_empty() {
            return Some(ForeignKeyReference {
                table_ident,
                column_ident,
            });
        }
    } else {
        // Path doesn't match expected pattern
    }
    None
}
