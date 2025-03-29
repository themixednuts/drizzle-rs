use syn::{
    Attribute, Expr, ExprClosure, ExprPath, Field, Ident, Lit, LitStr, Meta, Token, Type, TypePath,
    punctuated::Punctuated,
};

// List of supported SQLite column types as attribute names
const SQLITE_TYPES: [&str; 5] = ["integer", "text", "blob", "real", "number"];

#[derive(Clone)]
pub(crate) struct TableField<'a> {
    pub(crate) ident: &'a Ident,
    pub(crate) field: &'a Field,
    pub(crate) attrs: FieldAttributes,
}

#[derive(Default, Clone)]
pub(crate) struct FieldAttributes {
    pub(crate) name: Option<String>,
    pub(crate) primary_key: Option<Ident>,
    pub(crate) not_null: Option<Ident>,
    pub(crate) default_value: Option<Expr>,
    pub(crate) default_fn: Option<ExprClosure>,
    pub(crate) autoincrement: Option<Ident>,
    pub(crate) unique: Option<Ident>,
    pub(crate) references_path: Option<ExprPath>,
    pub(crate) column_type: Option<String>,
}

#[derive(Clone)]
pub(crate) enum Relationship {
    One(LitStr),
    Many(LitStr),
}

impl<'a> TryFrom<&'a Vec<Attribute>> for FieldAttributes {
    type Error = syn::Error;

    fn try_from(value: &'a Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut attrs = Self::default();

        for attr in value {
            // Check if the attribute path is one of our supported column types
            let path_ident = attr.path().get_ident();
            if let Some(ident) = path_ident {
                let type_name = ident.to_string();

								let type_name_str = type_name.as_str();

                // Check if this is a column type attribute
                if SQLITE_TYPES.contains(&type_name_str) {
                    // Set the column type
                    attrs.column_type = Some(type_name.clone());

                    // Handle the case of an empty attribute (e.g., #[text])
                    if let Meta::Path(_) = attr.meta {
                        // This is an attribute without arguments, like #[text]
                        continue;
                    }

                    // Parse the arguments for this type attribute
                    attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
                        .iter()
                        .try_for_each(|expr| -> Result<(), syn::Error> {
                            match expr {
                                Expr::Path(path_expr) => {
                                    // Handle flags like primary_key, not_null, etc.
                                    if let Some(flag_ident) = path_expr.path.get_ident() {
                                        let flag_str = flag_ident.to_string();

                                        match flag_str.as_str() {
                                            "primary_key" | "primary" => {
                                                attrs.primary_key = Some(flag_ident.clone());
                                            }
                                            "not_null" => {
                                                attrs.not_null = Some(flag_ident.clone());
                                            }
                                            "autoincrement" => {
    											if type_name_str != "integer" {
    												return Err(syn::Error::new_spanned(flag_ident, "autoincrement can be only used with the '#[integer] attribute"))
    											}
																							
                                                attrs.autoincrement = Some(flag_ident.clone());
                                            }
                                            "unique" => {
                                                attrs.unique = Some(flag_ident.clone());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Expr::Assign(assign_expr) => {
                                    // Handle named parameters (e.g., default = "value")
                                    if let Expr::Path(path_expr) = &*assign_expr.left {
                                        if let Some(param_ident) = path_expr.path.get_ident() {
                                            let param_name = param_ident.to_string();

                                            match param_name.as_str() {
                                                "name" => {
                                                    if let Expr::Lit(lit_expr) = &*assign_expr.right
                                                    {
                                                        if let Lit::Str(lit_str) = &lit_expr.lit {
                                                            attrs.name = Some(lit_str.value());
                                                        }
                                                    }
                                                }
                                                "default" => {
                                                    attrs.default_value =
                                                        Some(*assign_expr.right.clone());
                                                }
                                                "default_fn" => {
                                                    if let Expr::Closure(path) = &*assign_expr.right
                                                    {
                                                        attrs.default_fn = Some(path.clone());
                                                    }
                                                }
                                                "references" => {
                                                    // Only handle path-based references (Table::column)
                                                    if let Expr::Path(path_expr) = &*assign_expr.right
                                                    {
                                                        attrs.references_path = Some(path_expr.clone());
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }

                            Ok(())
                        })?;

                    continue;
                }
            }
        }

        Ok(attrs)
    }
}

// Helper function to check if a type is an Option<T>
pub(crate) fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

// Helper function to get the inner type of Option<T>
pub(crate) fn get_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}
