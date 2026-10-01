//! Threads an alias tag through generated aliased-table code.
//!
//! Aliased columns (`users::AliasedId`) and the aliased table struct
//! (`AliasedUsers`) are generic over the alias tag, so scope checks can tell
//! `u1.id` from `u2.id` in a self-join. The alias generators write them as bare
//! paths; this pass adds the tag parameter to every impl over those types and
//! to every bare mention of them inside it.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::visit_mut::{self, VisitMut};
use syn::{GenericParam, Ident, Item, Path, PathArguments, TypePath, parse_quote};

/// Name of the tag parameter added to impls over aliased types.
const TAG_PARAM: &str = "__DrizzleAliasTag";

fn path_key(path: &Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn same_bare_path(path: &Path, targets: &[Vec<String>]) -> bool {
    let key = path_key(path);
    let bare = path
        .segments
        .last()
        .is_some_and(|s| matches!(s.arguments, PathArguments::None));
    // Match on the trailing segments, so `module::Aliased` matches with or
    // without a leading path qualifier.
    bare && targets.iter().any(|t| key.ends_with(t))
}

struct AddTag<'a> {
    targets: &'a [Vec<String>],
    tag: Ident,
}

impl VisitMut for AddTag<'_> {
    fn visit_type_path_mut(&mut self, node: &mut TypePath) {
        if node.qself.is_none() && same_bare_path(&node.path, self.targets) {
            let tag = &self.tag;
            if let Some(last) = node.path.segments.last_mut() {
                last.arguments = PathArguments::AngleBracketed(parse_quote!(<#tag>));
            }
            return;
        }
        visit_mut::visit_type_path_mut(self, node);
    }
}

fn mentions(item: &impl ToTokens, targets: &[Vec<String>]) -> bool {
    let text = item.to_token_stream().to_string().replace(' ', "");
    targets.iter().any(|t| text.contains(&t.join("::")) || text.contains(t.last().unwrap()))
}

/// Adds the alias tag to `tokens`. `aliased` lists the aliased column type
/// paths plus the aliased table struct path.
pub fn tag_aliased_items(tokens: TokenStream, aliased: &[TokenStream]) -> syn::Result<TokenStream> {
    let targets: Vec<Vec<String>> = aliased
        .iter()
        .map(|t| syn::parse2::<Path>(t.clone()).map(|p| path_key(&p)))
        .collect::<syn::Result<_>>()?;
    let mut file: syn::File = syn::parse2(tokens)?;
    let tag_param = Ident::new(TAG_PARAM, proc_macro2::Span::call_site());

    for item in &mut file.items {
        match item {
            Item::Impl(imp) => {
                let self_is_aliased = matches!(&*imp.self_ty, syn::Type::Path(tp)
                    if tp.qself.is_none() && same_bare_path(&tp.path, &targets));
                let existing_tag = imp.generics.params.iter().find_map(|p| match p {
                    GenericParam::Type(t) if t.ident == "Tag" => Some(t.ident.clone()),
                    _ => None,
                });
                let tag = if !self_is_aliased && !mentions(imp, &targets) {
                    None
                } else if let (false, Some(tag)) = (self_is_aliased, existing_tag) {
                    Some(tag)
                } else {
                    imp.generics
                        .params
                        .push(GenericParam::Type(parse_quote!(#tag_param: drizzle::core::Tag)));
                    Some(tag_param.clone())
                };
                if let Some(tag) = tag {
                    AddTag { targets: &targets, tag }.visit_item_impl_mut(imp);
                }
            }
            Item::Struct(st) => {
                let is_target = targets
                    .iter()
                    .any(|t| t.len() == 1 && st.ident == t[0].as_str());
                if is_target {
                    st.generics.params.push(GenericParam::Type(parse_quote!(#tag_param: drizzle::core::Tag)));
                    AddTag { targets: &targets, tag: tag_param.clone() }.visit_item_struct_mut(st);
                } else if let Some(tag) = st.generics.params.iter().find_map(|p| match p {
                    GenericParam::Type(t) if t.ident == "Tag" => Some(t.ident.clone()),
                    _ => None,
                }) {
                    AddTag { targets: &targets, tag }.visit_item_struct_mut(st);
                }
            }
            _ => {}
        }
    }
    Ok(quote!(#file))
}
