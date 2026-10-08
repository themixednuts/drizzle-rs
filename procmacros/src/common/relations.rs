//! Relations for the relational query API, planned from a table's keys.
//!
//! Every foreign key gives two accessors: a forward one on the declaring
//! table that loads the referenced row, and a reverse one on the referenced
//! table that loads the declaring table's rows. A link table also gives each
//! of its two referenced tables a many-to-many accessor to the other.
//!
//! Each table macro sees only its own table, so names are built to be unique
//! from what it sees, and schemas need no annotation:
//!
//! - forward: the column without its `_id` suffix (`author_id` gives
//!   `author`); a composite key takes the referenced struct's singular name.
//! - reverse: the plural of the declaring struct (`posts`), or its singular
//!   when the key alone is unique, which also loads an `Option`. A column
//!   named for a role rather than the table it references prefixes the role
//!   (`author_id` to `Users` gives `users.author_posts()`), so the name
//!   depends on that column alone. `relation = "..."` names it.
//! - many-to-many: the plural of the other key's forward name. A link table
//!   named only after the tables and columns it links gives that (`posts.tags()`
//!   through `PostTags`); any other link adds its own name (`users.posts_via_likes()`
//!   through `PostLikes`), so two links between the same tables never clash.
//!   `many_to_many = "..."` names it.
//!
//! The table macro reports clashes among the accessors it generates.
//! Accessors two tables give a third clash in rustc, which reports the
//! duplicate at both declaring columns.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::Visibility;
use syn::ext::IdentExt;

use super::constraints::{
    CompositeForeignKeyRef, ConstraintFieldInfo, DialectTypes, ForeignKeyRef, RelationNames,
    cross_table_column_name_const,
};

/// One foreign key, from a column's `references` or a table's
/// `foreign_key(...)`.
struct ForeignKey {
    /// `(declaring column, referenced column)` pairs in key order: the
    /// declaring column's SQL name, and a constant expression for the
    /// referenced one, which reads the name that column declares.
    columns: Vec<(String, TokenStream)>,
    /// The referenced table.
    target: Ident,
    /// Whether a declaring column is nullable, so the referenced row may be
    /// absent.
    is_nullable: bool,
    /// Names given in the declaration.
    names: RelationNames,
    /// The forward accessor's name.
    forward: String,
    /// Where the key is declared. Accessors through it are reported here, so
    /// a clash points at the declaration to name.
    span: Span,
}

impl ForeignKey {
    /// The declaring columns.
    fn source_columns(&self) -> impl Iterator<Item = &str> {
        self.columns.iter().map(|(source, _)| source.as_str())
    }

    /// The role the key's column plays, when it is not named after the table
    /// it references: `author` for `author_id` to `Users`, but none for
    /// `user_id`, or for `user_id` to `AppUsers`.
    fn role(&self) -> Option<&str> {
        let target = singular(&self.target.to_string().to_snake_case());
        let forward = singular(&self.forward);
        let named_after_target = target == forward || target.ends_with(&format!("_{forward}"));
        (!named_after_target).then_some(self.forward.as_str())
    }

    /// The `(a, b)` pairs of a relation that loads the referenced table from
    /// the declaring one, as a `&'static` slice.
    fn forward_pairs(&self) -> TokenStream {
        let pairs = self
            .columns
            .iter()
            .map(|(source, target)| quote!((#target, #source)));
        const_pairs(pairs)
    }

    /// The `(a, b)` pairs of a relation that loads the declaring table from
    /// the referenced one, as a `&'static` slice.
    fn reverse_pairs(&self) -> TokenStream {
        let pairs = self
            .columns
            .iter()
            .map(|(source, target)| quote!((#source, #target)));
        const_pairs(pairs)
    }
}

/// What relation planning needs from a table: its columns, foreign keys and
/// the column sets that identify a row.
pub struct TableKeys {
    /// Every column by SQL name, in declaration order.
    columns: Vec<String>,
    /// Column-level keys in field order, then table-level ones.
    foreign_keys: Vec<ForeignKey>,
    /// The primary key's columns; empty when the table has none.
    primary_key: Vec<String>,
    /// The primary key, then each unique constraint, column- or table-level.
    row_keys: Vec<Vec<String>>,
}

impl TableKeys {
    /// Collects the keys of a table's fields, its table-level foreign keys
    /// and its table-level unique constraints.
    ///
    /// A table-level column that names no field is skipped here; the
    /// constraint generation reports it.
    ///
    /// `dialect` resolves referenced columns' SQL names at compile time.
    pub fn new<'c, F: ConstraintFieldInfo, C: CompositeForeignKeyRef>(
        fields: &[F],
        composite_foreign_keys: &[C],
        unique_constraints: impl IntoIterator<Item = &'c [Ident]>,
        dialect: &DialectTypes,
    ) -> Self {
        let field = |ident: &Ident| fields.iter().find(|field| field.ident() == ident);
        let column_names = |idents: &[Ident]| -> Option<Vec<String>> {
            idents
                .iter()
                .map(|ident| field(ident).map(|field| field.column_name().to_owned()))
                .collect()
        };

        let mut foreign_keys: Vec<ForeignKey> = fields
            .iter()
            .filter_map(|field| {
                let reference = field.foreign_key()?;
                let column = field.column_name().to_owned();
                Some(ForeignKey {
                    forward: forward_name(&column),
                    columns: vec![(
                        column,
                        cross_table_column_name_const(
                            reference.ref_table(),
                            reference.ref_column(),
                            dialect,
                        ),
                    )],
                    target: reference.ref_table().clone(),
                    is_nullable: field.is_nullable(),
                    names: field.relation_names(),
                    span: field.ident().span(),
                })
            })
            .collect();
        foreign_keys.extend(composite_foreign_keys.iter().filter_map(|key| {
            let sources = column_names(key.source_columns())?;
            let is_nullable = key
                .source_columns()
                .iter()
                .any(|ident| field(ident).is_some_and(ConstraintFieldInfo::is_nullable));
            let (forward, _) = composite_forward_names(key);
            Some(ForeignKey {
                columns: sources
                    .into_iter()
                    .zip(key.target_columns().iter().map(|column| {
                        cross_table_column_name_const(key.target_table(), column, dialect)
                    }))
                    .collect(),
                target: key.target_table().clone(),
                is_nullable,
                names: key.relation_names().clone(),
                forward,
                span: key
                    .source_columns()
                    .first()
                    .map_or_else(Span::call_site, Ident::span),
            })
        }));
        // A composite key whose name another key already has takes its full
        // column names instead, so single-column names never move.
        let column_keys = foreign_keys.len() - composite_foreign_keys.len();
        for (index, key) in composite_foreign_keys.iter().enumerate() {
            let position = column_keys + index;
            let Some(current) = foreign_keys.get(position).map(|key| key.forward.clone()) else {
                continue;
            };
            let taken = foreign_keys
                .iter()
                .enumerate()
                .any(|(other, key)| other != position && key.forward == current);
            if taken {
                foreign_keys[position].forward = composite_forward_names(key).1;
            }
        }

        let primary_key: Vec<String> = fields
            .iter()
            .filter(|field| field.is_primary())
            .map(|field| field.column_name().to_owned())
            .collect();
        let unique_columns = fields
            .iter()
            .filter(|field| field.is_unique())
            .map(|field| vec![field.column_name().to_owned()]);
        let unique_constraints = unique_constraints.into_iter().filter_map(column_names);
        let row_keys = std::iter::once(primary_key.clone())
            .filter(|key| !key.is_empty())
            .chain(unique_columns)
            .chain(unique_constraints)
            .collect();

        Self {
            columns: fields
                .iter()
                .map(|field| field.column_name().to_owned())
                .collect(),
            foreign_keys,
            primary_key,
            row_keys,
        }
    }

    /// Every column by SQL name, in declaration order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Whether `columns` identify a row: they are the primary key or a
    /// unique constraint, in any order.
    fn is_row_key<'a>(&self, columns: impl IntoIterator<Item = &'a str>) -> bool {
        let mut columns: Vec<&str> = columns.into_iter().collect();
        columns.sort_unstable();
        self.row_keys.iter().any(|key| {
            let mut key: Vec<&str> = key.iter().map(String::as_str).collect();
            key.sort_unstable();
            key == columns
        })
    }

    /// The table's two foreign keys, when it is a link table between them.
    ///
    /// A link table's rows are its pair of keys: the pair is the primary key
    /// or a unique constraint, or the table has no column besides the pair
    /// and a single-column primary key. Neither key may be unique alone,
    /// which would make the table a one-to-one extension of that side.
    /// `many_to_many` on either key makes a table with two keys a link.
    fn link_pair(&self, struct_ident: &Ident) -> Result<Option<[&ForeignKey; 2]>, TokenStream> {
        let opted_in = self
            .foreign_keys
            .iter()
            .find(|key| key.names.many_to_many.is_some());
        let pair = match self.foreign_keys.as_slice() {
            [a, b] if a.target != *struct_ident && b.target != *struct_ident => [a, b],
            _ => {
                let Some(key) = opted_in else {
                    return Ok(None);
                };
                let msg = format!(
                    "many_to_many needs a link table: exactly two foreign keys, both to other \
                     tables. `{struct_ident}` has {} foreign key(s){}.",
                    self.foreign_keys.len(),
                    if self
                        .foreign_keys
                        .iter()
                        .any(|key| key.target == *struct_ident)
                    {
                        ", one of them to itself"
                    } else {
                        ""
                    },
                );
                return Err(quote_spanned! {key.span=> ::core::compile_error!(#msg); });
            }
        };
        if opted_in.is_some() {
            return Ok(Some(pair));
        }

        let [a, b] = pair;
        if self.is_row_key(a.source_columns()) || self.is_row_key(b.source_columns()) {
            return Ok(None);
        }
        let pair_columns: Vec<&str> = a.source_columns().chain(b.source_columns()).collect();
        if self.is_row_key(pair_columns.iter().copied()) {
            return Ok(Some(pair));
        }
        let rest: Vec<&str> = self
            .columns
            .iter()
            .map(String::as_str)
            .filter(|column| !pair_columns.contains(column))
            .collect();
        let is_link = match rest.as_slice() {
            [] => true,
            [surrogate] => self.primary_key == [*surrogate],
            _ => false,
        };
        Ok(is_link.then_some(pair))
    }
}

/// Generates the relations a table's keys give the query API: forward and
/// reverse accessors per foreign key, and the many-to-many pair of a link
/// table.
pub fn generate_relations(struct_ident: &Ident, vis: &Visibility, keys: &TableKeys) -> TokenStream {
    let mut tokens = TokenStream::new();

    let mut accessors = plan_forward(struct_ident, keys);
    accessors.extend(plan_reverse(struct_ident, keys));
    match keys.link_pair(struct_ident) {
        Ok(Some(pair)) => accessors.extend(plan_many_to_many(struct_ident, pair)),
        Ok(None) => {}
        Err(error) => tokens.extend(error),
    }
    tokens.extend(drop_duplicates(&mut accessors));

    let rels_struct = format_ident!("__{struct_ident}ForwardRels");
    if accessors
        .iter()
        .any(|accessor| matches!(accessor.direction, Direction::Forward))
    {
        // Forward accessors live on a hidden struct reached through `Deref`
        // on the table ZST. This avoids collisions with the per-column
        // associated constants the table macro generates (such as
        // `const invited_by: InvitedByColumn`) while `table.relation()`
        // still needs no trait import.
        tokens.extend(quote! {
            #[doc(hidden)]
            #vis struct #rels_struct;

            impl ::std::ops::Deref for #struct_ident {
                type Target = #rels_struct;
                fn deref(&self) -> &#rels_struct {
                    &#rels_struct
                }
            }
        });
    }
    for accessor in &accessors {
        tokens.extend(accessor.emit(struct_ident, &rels_struct, vis));
    }
    tokens
}

/// Which way an accessor goes through its key.
enum Direction<'k> {
    /// On the declaring table, loading the referenced row.
    Forward,
    /// On the referenced table, loading the declaring table's rows; at most
    /// one when the key alone is unique.
    Reverse { one: bool },
    /// On the referenced table, loading the other key's table through this
    /// link table.
    ManyToMany { other: &'k ForeignKey },
}

/// An accessor a foreign key generates.
struct Accessor<'k> {
    /// The key the accessor goes through.
    key: &'k ForeignKey,
    direction: Direction<'k>,
    /// The table that gets the accessor.
    receiver: &'k Ident,
    /// The accessor's name.
    method: String,
}

impl Accessor<'_> {
    const fn kind(&self) -> &'static str {
        match self.direction {
            Direction::Forward => "forward",
            Direction::Reverse { .. } => "reverse",
            Direction::ManyToMany { .. } => "many-to-many",
        }
    }

    /// The declaring columns, for messages.
    fn columns(&self) -> String {
        self.key
            .source_columns()
            .map(|column| format!("`{column}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The attribute key that names this accessor; a forward accessor has
    /// none.
    const fn naming_key(&self) -> Option<&'static str> {
        match self.direction {
            Direction::Forward => None,
            Direction::Reverse { .. } => Some("relation"),
            Direction::ManyToMany { .. } => Some("many_to_many"),
        }
    }

    /// How to tell this accessor and `other`, which shares its name, apart.
    fn advice(&self, other: &Self) -> String {
        match (other.naming_key(), self.naming_key()) {
            (None, None) => "Rename one of the columns: a forward accessor is named after its \
                             column, or a composite key after the table it references."
                .to_owned(),
            (Some(a), Some(b)) if a == b => format!("Name one of them with `{a} = \"...\"`."),
            (Some(a), Some(b)) => {
                format!("Name one of them with `{a} = \"...\"` or `{b} = \"...\"`.")
            }
            (Some(key), None) | (None, Some(key)) => format!(
                "Rename the forward one's column, or name the other with `{key} = \"...\"`."
            ),
        }
    }

    /// Emits the accessor's relation; `struct_ident` declares the key, and
    /// `rels_struct` holds its forward accessors.
    fn emit(&self, struct_ident: &Ident, rels_struct: &Ident, vis: &Visibility) -> TokenStream {
        let key = self.key;
        let referenced = &key.target;
        let (loads, card, fk_columns, junction) = match self.direction {
            Direction::Forward => (
                referenced,
                if key.is_nullable {
                    Cardinality::OptionalOne
                } else {
                    Cardinality::One
                },
                key.forward_pairs(),
                None,
            ),
            Direction::Reverse { one } => (
                struct_ident,
                if one {
                    Cardinality::OptionalOne
                } else {
                    Cardinality::Many
                },
                key.reverse_pairs(),
                None,
            ),
            Direction::ManyToMany { other } => {
                let source_fk = key.reverse_pairs();
                let target_fk = other.reverse_pairs();
                (
                    &other.target,
                    Cardinality::Many,
                    quote!(&[]),
                    Some(quote! {
                        Some(drizzle::core::relation::JunctionMeta {
                            table: <#struct_ident as drizzle::core::query::QueryTable>::TABLE,
                            source_fk: #source_fk,
                            target_fk: #target_fk,
                        })
                    }),
                )
            }
        };
        let accessor_receiver = if matches!(self.direction, Direction::Forward) {
            rels_struct
        } else {
            self.receiver
        };
        RelEmitter {
            vis,
            declaring: struct_ident,
            receiver: self.receiver,
            method: &self.method,
            loads,
            card,
            fk_columns,
            junction,
            accessor_receiver,
            span: key.span,
        }
        .emit()
    }
}

/// Names the forward accessor of each key.
fn plan_forward<'k>(struct_ident: &'k Ident, keys: &'k TableKeys) -> Vec<Accessor<'k>> {
    keys.foreign_keys
        .iter()
        .map(|key| Accessor {
            key,
            direction: Direction::Forward,
            receiver: struct_ident,
            method: key.forward.clone(),
        })
        .collect()
}

/// Names the reverse accessor each key gives its referenced table:
/// `relation = "..."`, or the declaring struct's plural (its singular when
/// the key alone is unique), prefixed with the key's role.
fn plan_reverse<'k>(struct_ident: &Ident, keys: &'k TableKeys) -> Vec<Accessor<'k>> {
    let base = struct_ident.to_string().to_snake_case();
    keys.foreign_keys
        .iter()
        .map(|key| {
            let one = keys.is_row_key(key.source_columns());
            let method = key.names.relation.clone().unwrap_or_else(|| {
                let name = if one { singular(&base) } else { plural(&base) };
                key.role()
                    .map_or_else(|| name.clone(), |role| format!("{role}_{name}"))
            });
            Accessor {
                key,
                direction: Direction::Reverse { one },
                receiver: &key.target,
                method,
            }
        })
        .collect()
}

/// Names the many-to-many accessor each side of a link table gets to the
/// other: `many_to_many = "..."`, or the plural of the other key's forward
/// name, followed by the link's own name when it has one.
fn plan_many_to_many<'k>(struct_ident: &Ident, [a, b]: [&'k ForeignKey; 2]) -> Vec<Accessor<'k>> {
    let via =
        link_name(struct_ident, [a, b]).map_or_else(String::new, |name| format!("_via_{name}"));
    [(a, b), (b, a)]
        .into_iter()
        .map(|(key, other)| Accessor {
            key,
            direction: Direction::ManyToMany { other },
            receiver: &key.target,
            method: key
                .names
                .many_to_many
                .clone()
                .unwrap_or_else(|| format!("{}{via}", plural(&other.forward))),
        })
        .collect()
}

/// What a link table's name adds to the tables and columns it links, such as
/// `likes` for `PostLikes` between `Users` and `Posts`. `None` when the name
/// only combines them, as in `PostTags`, `UsersToGroups` or `GroupMembers`
/// with a `member_id` column.
///
/// A link with a name of its own means something particular (a like, a
/// bookmark), and two of them can join the same tables, so their
/// many-to-many accessors carry that name.
fn link_name(struct_ident: &Ident, [a, b]: [&ForeignKey; 2]) -> Option<String> {
    const CONNECTORS: &[&str] = &["to", "and", "x", "has", "with"];
    let linked: Vec<String> = [a, b]
        .into_iter()
        .flat_map(|key| {
            let target = key.target.to_string().to_snake_case();
            let forward = key.forward.clone();
            [target, forward]
        })
        .flat_map(|name| name.split('_').map(singular).collect::<Vec<_>>())
        .collect();
    let link = struct_ident.to_string().to_snake_case();
    let own: Vec<&str> = link
        .split('_')
        .filter(|word| !CONNECTORS.contains(word) && !linked.contains(&singular(word)))
        .collect();
    (!own.is_empty()).then(|| plural(&own.join("_")))
}

/// Drops each accessor that would give its table a name an earlier one
/// already gave it, and reports it at its key instead.
///
/// Forward accessors live on a different type than reverse and many-to-many
/// ones, but every relation's row struct is `{Receiver}With{Name}`, so any
/// two that share a receiver and a name clash.
fn drop_duplicates(accessors: &mut Vec<Accessor<'_>>) -> TokenStream {
    let mut tokens = TokenStream::new();
    let mut kept: Vec<Accessor<'_>> = Vec::with_capacity(accessors.len());
    for accessor in std::mem::take(accessors) {
        let Some(prev) = kept
            .iter()
            .find(|prev| prev.receiver == accessor.receiver && prev.method == accessor.method)
        else {
            kept.push(accessor);
            continue;
        };
        let advice = accessor.advice(prev);
        let msg = format!(
            "duplicate relation accessor `{}` on `{}`: the {} relation through {} and the {} \
             relation through {}. {advice}",
            accessor.method,
            accessor.receiver,
            prev.kind(),
            prev.columns(),
            accessor.kind(),
            accessor.columns(),
        );
        tokens.extend(quote_spanned! {accessor.key.span=> ::core::compile_error!(#msg); });
    }
    *accessors = kept;
    tokens
}

/// Cardinality of a generated relation; sets the with-row field type.
#[derive(Clone, Copy)]
enum Cardinality {
    Many,
    One,
    OptionalOne,
}

/// Emits one relation: its ZST, `RelationDef` and `AssembleRel` impls, its
/// `{Receiver}With{Method}` row struct, and its accessor method.
struct RelEmitter<'a> {
    /// `pub` / `pub(crate)` from the host struct.
    vis: &'a Visibility,
    /// The table whose macro generates the relation.
    declaring: &'a Ident,
    /// The table that gets the accessor.
    receiver: &'a Ident,
    /// The accessor's name, also the row struct's field and the relation's
    /// JSON key.
    method: &'a str,
    /// The table the relation loads.
    loads: &'a Ident,
    card: Cardinality,
    /// Body of `fn fk_columns()`.
    fk_columns: TokenStream,
    /// Body of `fn junction()` for a many-to-many relation.
    junction: Option<TokenStream>,
    /// The type whose inherent impl holds the accessor method.
    accessor_receiver: &'a Ident,
    /// The key's declaration; duplicate definitions are reported here.
    span: Span,
}

impl RelEmitter<'_> {
    fn emit(&self) -> TokenStream {
        let RelEmitter {
            vis,
            declaring,
            receiver,
            method,
            loads,
            card,
            fk_columns,
            junction,
            accessor_receiver,
            span,
        } = self;
        let span = Span::call_site().located_at(*span);
        let pascal = method.to_upper_camel_case();
        // The ZST and row struct name the declaring table when it is not the
        // receiver, so they stay unique. When two tables give the receiver
        // one name, only the accessor and the public alias clash, at both
        // declarations.
        let by = if declaring == receiver {
            String::new()
        } else {
            format!("_By{declaring}")
        };
        let rel_zst = format_ident!("__Rel_{receiver}_{pascal}{by}");
        let row_struct = format_ident!("__Row_{receiver}_{pascal}{by}");
        let row_name = format!("{receiver}With{pascal}");
        let row_alias = Ident::new(&row_name, span);
        // A column such as `type_id` gives the keyword `type`.
        let method_ident = if syn::parse_str::<Ident>(method).is_ok() {
            Ident::new(method, span)
        } else {
            Ident::new_raw(method, span)
        };
        let receiver_select = format_ident!("Select{receiver}");
        let loads_select = format_ident!("Select{loads}");

        let (card, field_ty) = match card {
            Cardinality::Many => (
                quote!(drizzle::core::relation::Many),
                quote!(::std::vec::Vec<Child>),
            ),
            Cardinality::One => (quote!(drizzle::core::relation::One), quote!(Child)),
            Cardinality::OptionalOne => (
                quote!(drizzle::core::relation::OptionalOne),
                quote!(::std::option::Option<Child>),
            ),
        };
        let junction_fn = junction.as_ref().map(|body| {
            quote! {
                fn junction() -> Option<drizzle::core::relation::JunctionMeta> { #body }
            }
        });
        let row_doc = format!(
            "Query result row with the `{method}` relation loaded.\n\n\
             Base columns are available through [`Deref`](core::ops::Deref). The `{method}` \
             field holds the loaded relation. Compose multiple relations by nesting another \
             `*With*` type as `Inner`."
        );
        let field_doc = format!("Loaded `{method}` relation.");

        quote_spanned! {span=>
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy)]
            #[allow(non_camel_case_types)]
            #vis struct #rel_zst;

            impl drizzle::core::relation::private::Sealed for #rel_zst {}

            impl drizzle::core::relation::RelationDef for #rel_zst {
                type Source = #receiver;
                type Target = #loads;
                type Card = #card;
                const NAME: &'static str = #method;
                fn fk_columns() -> &'static [(&'static str, &'static str)] {
                    #fk_columns
                }
                #junction_fn
            }

            #[doc = #row_doc]
            #vis type #row_alias<Inner = #receiver_select, Child = #loads_select> =
                #row_struct<Inner, Child>;

            #[doc(hidden)]
            #[derive(Clone)]
            #[allow(non_camel_case_types)]
            #vis struct #row_struct<Inner, Child> {
                /// Inner row — the root select model, or a previously composed with-row.
                pub inner: Inner,
                #[doc = #field_doc]
                pub #method_ident: #field_ty,
            }

            impl<Inner: ::core::fmt::Debug, Child: ::core::fmt::Debug> ::core::fmt::Debug
                for #row_struct<Inner, Child>
            {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct(#row_name)
                        .field("inner", &self.inner)
                        .field(#method, &self.#method_ident)
                        .finish()
                }
            }

            impl<Inner, Child> ::core::ops::Deref for #row_struct<Inner, Child> {
                type Target = Inner;
                #[inline]
                fn deref(&self) -> &Inner {
                    &self.inner
                }
            }

            impl drizzle::core::relation::AssembleRel for #rel_zst {
                type Row<Inner, Child> = #row_struct<Inner, Child>;

                #[inline]
                fn assemble_row<Inner, Child>(
                    inner: Inner,
                    data: <Self::Card as drizzle::core::relation::CardWrap>::Wrap<Child>,
                ) -> Self::Row<Inner, Child> {
                    #row_struct {
                        inner,
                        #method_ident: data,
                    }
                }
            }

            impl #accessor_receiver {
                #vis fn #method_ident<'a, __V: drizzle::core::SQLParam>(&self) -> drizzle::core::query::RelationHandle<'a, __V, #rel_zst> {
                    drizzle::core::query::RelationHandle::new()
                }
            }
        }
    }
}

/// A `&'static [(&str, &str)]` of `pairs`, through a constant so the names
/// may be constant expressions.
fn const_pairs(pairs: impl Iterator<Item = TokenStream>) -> TokenStream {
    quote! {
        {
            const PAIRS: &[(&str, &str)] = &[#(#pairs),*];
            PAIRS
        }
    }
}

/// The forward accessor names of a composite key: the name it takes, and the
/// longer one it falls back to when another key on the table has that name.
///
/// Columns that share the referenced column's name (`tenant` to `tenant`) say
/// nothing about the key and are left out. The rest name it as a
/// single-column key would, without the referenced column's name:
/// `parent_code` to `code` gives `parent`. With nothing left, the key takes
/// the referenced struct's singular name. The fallback keeps those columns
/// whole (`parent_code`).
fn composite_forward_names<C: CompositeForeignKeyRef>(key: &C) -> (String, String) {
    let target_name = singular(&key.target_table().to_string().to_snake_case());
    let distinct: Vec<(String, String)> = key
        .source_columns()
        .iter()
        .zip(key.target_columns())
        .map(|(source, target)| (source.unraw().to_string(), target.unraw().to_string()))
        .filter(|(source, target)| source != target)
        .collect();
    if distinct.is_empty() {
        return (target_name.clone(), target_name);
    }
    let short = distinct
        .iter()
        .map(|(source, target)| {
            let source = forward_name(source);
            source
                .strip_suffix(&format!("_{target}"))
                .map_or(source.clone(), ToOwned::to_owned)
        })
        .collect::<Vec<_>>()
        .join("_");
    let long = distinct
        .iter()
        .map(|(source, _)| source.as_str())
        .collect::<Vec<_>>()
        .join("_");
    (short, long)
}

/// The forward accessor name of a single-column key: the column without its
/// `_id` suffix (`author_id` gives `author`), or the column itself.
fn forward_name(column: &str) -> String {
    match column.strip_suffix("_id") {
        Some(stripped) if !stripped.is_empty() => stripped.to_owned(),
        _ => column.to_owned(),
    }
}

/// `post` gives `posts`, `category` gives `categories`.
fn plural(word: &str) -> String {
    pluralizer::pluralize(word, 2, false)
}

/// `users` gives `user`, `categories` gives `category`.
fn singular(word: &str) -> String {
    pluralizer::pluralize(word, 1, false)
}
