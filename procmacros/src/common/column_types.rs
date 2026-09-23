//! Where a table's or view's generated column types live.
//!
//! `#[SQLiteTable] struct Users { id: i64, name: String }` puts one unit
//! struct per column in a module named after the table in snake case:
//! `users::Id` and `users::Name`. The same module holds the alias columns
//! (`users::AliasedName`) and the insert model's typestate markers
//! (`users::NameSet`, `users::NameNotSet`).
//!
//! These names used to sit beside the table as `UsersName`, where a column
//! could collide with the user's own types: `User.role` generated `UserRole`,
//! the natural name for the enum that column stores.
//!
//! Only the struct definitions go inside the module. Their impls stay beside
//! the table and name them by path, because a table declared inside a
//! function body is invisible to a nested module.
//!
//! Every name here takes the table's span, as the flat names did, so
//! diagnostics about a column still point at the table.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::fmt::Display;

/// The module holding `table`'s column types: its name in snake case, raw or
/// suffixed when that is a keyword.
pub fn columns_module(table: &Ident) -> Ident {
    let name = table.to_string().to_snake_case();
    if syn::parse_str::<Ident>(&name).is_ok() {
        return Ident::new(&name, table.span());
    }
    match name.as_str() {
        // Keywords that can't be raw identifiers.
        "crate" | "self" | "super" => format_ident!("{}_", name, span = table.span()),
        _ => Ident::new_raw(&name, table.span()),
    }
}

/// The field's name in upper camel case, as every generated per-column type
/// spells it.
fn pascal(field: &impl Display) -> String {
    field.to_string().to_upper_camel_case()
}

/// The column type's name inside `table`'s module: `Name` for `name`.
pub fn column_type_name(table: &Ident, field: &impl Display) -> Ident {
    match pascal(field).as_str() {
        // `self_` would name its column `Self`.
        "Self" => format_ident!("Self_", span = table.span()),
        name => Ident::new(name, table.span()),
    }
}

/// The path to a column type from beside its table: `users::Name`.
pub fn column_type(table: &Ident, field: &impl Display) -> TokenStream {
    let module = columns_module(table);
    let name = column_type_name(table, field);
    quote!(#module::#name)
}

/// The alias column type's name inside the module: `AliasedName`.
pub fn aliased_column_type_name(table: &Ident, field: &impl Display) -> Ident {
    format_ident!("Aliased{}", pascal(field), span = table.span())
}

/// The path to an alias column type: `users::AliasedName`.
pub fn aliased_column_type(table: &Ident, field: &impl Display) -> TokenStream {
    let module = columns_module(table);
    let name = aliased_column_type_name(table, field);
    quote!(#module::#name)
}

fn set_marker_name(table: &Ident, field: &impl Display) -> Ident {
    format_ident!("{}Set", pascal(field), span = table.span())
}

fn not_set_marker_name(table: &Ident, field: &impl Display) -> Ident {
    format_ident!("{}NotSet", pascal(field), span = table.span())
}

/// The insert model's marker for a field that has been set: `users::NameSet`.
pub fn set_marker(table: &Ident, field: &impl Display) -> TokenStream {
    let module = columns_module(table);
    let name = set_marker_name(table, field);
    quote!(#module::#name)
}

/// The insert model's marker for a field not set yet: `users::NameNotSet`.
pub fn not_set_marker(table: &Ident, field: &impl Display) -> TokenStream {
    let module = columns_module(table);
    let name = not_set_marker_name(table, field);
    quote!(#module::#name)
}

/// A name for the generic parameter that stands for `field`'s insert state
/// in the `with_*` setters. It must not be a type the user could name: the
/// setter's signature also spells the field's own type, which a parameter
/// named `UserRole` would capture.
pub fn insert_state_param(table: &Ident, field: &impl Display) -> Ident {
    format_ident!("__{}", pascal(field), span = table.span())
}

/// `vis`, the table's visibility, spelled from inside the column module.
///
/// A column type must be nominally no more visible than its table: its trait
/// impls name the table (`type Table = Users`), and a `pub` type there would
/// put a private table in a public interface. Restricted visibilities that
/// count from the current module need one more `super`.
fn visibility_one_level_down(vis: &syn::Visibility) -> TokenStream {
    match vis {
        syn::Visibility::Public(_) => quote!(pub),
        syn::Visibility::Inherited => quote!(pub(super)),
        syn::Visibility::Restricted(restricted) => {
            let path = &restricted.path;
            match path
                .segments
                .first()
                .map(|segment| segment.ident.to_string())
                .as_deref()
            {
                Some("self") => {
                    let rest = path.segments.iter().skip(1);
                    quote!(pub(in super #(::#rest)*))
                }
                Some("super") => quote!(pub(in super::#path)),
                _ => quote!(#vis),
            }
        }
    }
}

/// The module itself, with every per-column struct of `table`.
///
/// The module carries the table's visibility. The column and alias types
/// inside are as visible as the table (see [`visibility_one_level_down`]).
/// The insert markers are `pub`: the insert model is always `pub` and names
/// them in its default type parameter, and they name nothing private.
pub fn generate_columns_module(
    table: &Ident,
    vis: &syn::Visibility,
    fields: &[&Ident],
) -> TokenStream {
    let module = columns_module(table);
    let doc = format!(" Column types of `{table}`.");
    let item_vis = visibility_one_level_down(vis);
    let items = fields.iter().map(|field| {
        let column = column_type_name(table, field);
        let aliased = aliased_column_type_name(table, field);
        let set = set_marker_name(table, field);
        let not_set = not_set_marker_name(table, field);
        quote! {
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #item_vis struct #column;

            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #item_vis struct #aliased {
                pub(super) alias: &'static str,
            }

            pub struct #set;
            pub struct #not_set;
        }
    });
    quote! {
        #[doc = #doc]
        #[allow(non_camel_case_types, dead_code)]
        #vis mod #module {
            #(#items)*
        }
    }
}
