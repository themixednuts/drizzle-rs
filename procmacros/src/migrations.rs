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

    let discovered = drizzle_migrations::MigrationDir::new(&migrations_dir)
        .discover()
        .map_err(|e| syn::Error::new(Span::call_site(), e.to_string()))?;
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
