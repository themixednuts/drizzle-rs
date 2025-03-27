use syn::{punctuated::Punctuated, Attribute, Expr, ExprAssign, ExprLit, ExprPath, Lit, Token};

const TABLE: &str = "table";

#[derive(Default, Clone)]
pub(crate) struct TableAttributes {
    pub(crate) name: Option<String>,
    pub(crate) strict: bool,
    pub(crate) without_rowid: bool,
}

impl<'a> TryFrom<&'a Vec<Attribute>> for TableAttributes {
    type Error = syn::Error;

    fn try_from(value: &'a Vec<Attribute>) -> Result<Self, Self::Error> {
        let mut tbl = TableAttributes::default();

        for attr in value {
            if !attr.path().is_ident(TABLE) {
                continue;
            }

            attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
                .iter()
                .try_for_each(|expr| -> Result<(), syn::Error> {
                    match expr {
                        Expr::Path(path_expr) => {
                            // Handle simple flags: strict, without_rowid
                            if let Some(ident) = path_expr.path.get_ident() {
                                let ident_str = ident.to_string();

                                match ident_str.as_str() {
                                    "strict" => {
                                        tbl.strict = true;
                                    }
                                    "without_rowid" => {
                                        tbl.without_rowid = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Expr::Assign(assign_expr) => {
                            // Handle key-value attributes: name
                            if let Expr::Path(path_expr) = &*assign_expr.left {
                                if let Some(ident) = path_expr.path.get_ident() {
                                    let ident_str = ident.to_string();

                                    match ident_str.as_str() {
                                        "name" => {
                                            if let Expr::Lit(lit_expr) = &*assign_expr.right {
                                                if let Lit::Str(lit_str) = &lit_expr.lit {
                                                    tbl.name = Some(lit_str.value());
                                                }
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
        }

        Ok(tbl)
    }
}
