use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::path::{Path, PathBuf};
use syn::LitStr;

pub fn include_migrations_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let path_lit: LitStr = syn::parse2(input)?;
    let path_value = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            "include_migrations!: CARGO_MANIFEST_DIR is not set",
        )
    })?;
    let manifest_dir = PathBuf::from(manifest_dir);
    let migrations_dir = manifest_dir.join(path_value);

    // Embedding no migrations would compile, then leave every database
    // unmigrated at runtime; a wrong path is a compile error instead.
    if !migrations_dir.is_dir() {
        return Err(syn::Error::new(
            path_lit.span(),
            format!(
                "include_migrations!: no migrations directory at `{}` \
                 (the path is relative to the crate's Cargo.toml; \
                 `drizzle generate` creates it)",
                migrations_dir.display()
            ),
        ));
    }

    let discovered = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .map_err(|e| {
            syn::Error::new(
                path_lit.span(),
                format!(
                    "include_migrations!: cannot read `{}`: {e}",
                    migrations_dir.display()
                ),
            )
        })?;
    let sql_paths = resolve_sql_paths(&migrations_dir, &discovered)?;
    let migration_ty = crate::paths::migrations::migration();

    let entries = discovered
        .iter()
        .zip(sql_paths.iter())
        .map(|(migration, sql_path)| {
            let include_path = include_path_expr(&manifest_dir, sql_path);
            let tag = LitStr::new(migration.tag(), Span::call_site());
            let hash = LitStr::new(migration.hash(), Span::call_site());
            let created_at = migration.created_at();
            let statements = migration
                .statements()
                .iter()
                .map(|stmt| LitStr::new(stmt, Span::call_site()));

            quote! {
                {
                    let _ = include_str!(#include_path);
                    #migration_ty::with_hash(
                        #tag,
                        #hash,
                        #created_at,
                        vec![#(#statements.to_string()),*],
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {{
        vec![#(#entries),*]
    }})
}

fn resolve_sql_paths(
    dir: &Path,
    migrations: &[drizzle_migrations::Migration],
) -> syn::Result<Vec<PathBuf>> {
    let mut paths = Vec::with_capacity(migrations.len());

    for migration in migrations {
        let tag = migration.tag();
        let path = dir.join(tag).join("migration.sql");
        if !path.exists() {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("include_migrations!: missing migration.sql for tag '{tag}'"),
            ));
        }

        paths.push(path);
    }

    Ok(paths)
}

fn include_path_expr(manifest_dir: &Path, sql_path: &Path) -> TokenStream {
    if let Ok(relative) = sql_path.strip_prefix(manifest_dir) {
        let relative = relative.to_string_lossy().replace('\\', "/");
        let suffix = LitStr::new(&format!("/{relative}"), Span::call_site());
        return quote!(concat!(env!("CARGO_MANIFEST_DIR"), #suffix));
    }

    let absolute = LitStr::new(&sql_path.to_string_lossy(), Span::call_site());
    quote!(#absolute)
}

#[cfg(test)]
mod tests {
    use super::include_migrations_impl;

    #[test]
    fn a_missing_directory_is_an_error_that_names_it() {
        let message = include_migrations_impl(quote::quote!("./no/such/migrations"))
            .expect_err("a missing directory must not embed an empty list")
            .to_string();
        assert!(message.contains("no migrations directory at"), "{message}");
        assert!(message.contains("migrations"), "{message}");
    }
}
