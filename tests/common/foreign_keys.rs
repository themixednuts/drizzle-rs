/// Foreign-key declarations, referential actions and constraint metadata.
///
/// `ON DELETE SET DEFAULT` is deliberately absent: MySQL's InnoDB parses but
/// rejects it, so SQLite and PostgreSQL keep that case in their own files.
macro_rules! shared_foreign_key_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_foreign_keys {
            use super::*;
            use drizzle::core::expr::eq;

            #[$table(NAME = "shared_fk_parents")]
            struct FkParent {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_fk_cascade")]
            struct FkCascade {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_DELETE = CASCADE)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_set_null")]
            struct FkSetNull {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_DELETE = SET_NULL)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_restrict")]
            struct FkRestrict {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_DELETE = RESTRICT)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_no_action")]
            struct FkNoAction {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_DELETE = NO_ACTION)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_update_cascade")]
            struct FkUpdateCascade {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_UPDATE = CASCADE)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_update_set_null")]
            struct FkUpdateSetNull {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_UPDATE = SET_NULL)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_both_actions")]
            struct FkBothActions {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = FkParent::id, ON_DELETE = CASCADE, ON_UPDATE = SET_NULL)]
                parent_id: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_composite_parents")]
            struct CompositeFkParent {
                // DEFAULT keeps both halves of the key optional in the insert
                // model on every dialect (PostgreSQL would otherwise require
                // them positionally while SQLite would not).
                #[column(PRIMARY, DEFAULT = 0)]
                id_a: i32,
                #[column(PRIMARY, DEFAULT = 0)]
                id_b: i32,
                label: String,
            }

            #[$table(
                NAME = "shared_fk_composite_children",
                FOREIGN_KEY(
                    columns(parent_a, parent_b),
                    references(CompositeFkParent, id_a, id_b),
                    on_delete = "CASCADE",
                    on_update = "CASCADE"
                )
            )]
            struct CompositeFkChild {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                parent_a: Option<i32>,
                parent_b: Option<i32>,
                value: String,
            }

            #[$table(NAME = "shared_fk_parents_custom")]
            struct NamedFkParent {
                #[column(PRIMARY, DEFAULT = 0, NAME = "parent_pk")]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_fk_children_custom")]
            struct NamedFkChild {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(NAME = "parent_ref", REFERENCES = NamedFkParent::id)]
                parent_id: Option<i32>,
            }

            #[derive($schema)]
            struct SharedFkSchema {
                parents: FkParent,
                cascade: FkCascade,
                set_null: FkSetNull,
                restrict: FkRestrict,
                no_action: FkNoAction,
                update_cascade: FkUpdateCascade,
                update_set_null: FkUpdateSetNull,
                both_actions: FkBothActions,
                composite_parents: CompositeFkParent,
                composite_children: CompositeFkChild,
                named_parents: NamedFkParent,
                named_children: NamedFkChild,
            }

            // -------------------------------------------------------- rendering

            #[test]
            fn referential_actions_render_in_create_table() {
                for (sql, expected) in [
                    (FkCascade::create_table_sql(), "ON DELETE CASCADE"),
                    (FkSetNull::create_table_sql(), "ON DELETE SET NULL"),
                    (FkRestrict::create_table_sql(), "ON DELETE RESTRICT"),
                    (FkUpdateCascade::create_table_sql(), "ON UPDATE CASCADE"),
                    (FkUpdateSetNull::create_table_sql(), "ON UPDATE SET NULL"),
                    (FkBothActions::create_table_sql(), "ON DELETE CASCADE"),
                    (FkBothActions::create_table_sql(), "ON UPDATE SET NULL"),
                ] {
                    assert!(sql.contains(expected), "expected `{expected}` in: {sql}");
                }

                // NO ACTION is the default and may be left implicit; the
                // constraint itself must still be there.
                let no_action = FkNoAction::create_table_sql();
                assert!(
                    no_action.contains("FOREIGN KEY") && no_action.contains("REFERENCES"),
                    "missing foreign key: {no_action}"
                );

                let composite = CompositeFkChild::create_table_sql();
                for expected in [
                    "FOREIGN KEY",
                    "parent_a",
                    "parent_b",
                    "REFERENCES",
                    "id_a",
                    "id_b",
                    "ON DELETE CASCADE",
                    "ON UPDATE CASCADE",
                ] {
                    assert!(
                        composite.contains(expected),
                        "expected `{expected}` in: {composite}"
                    );
                }
            }

            // --------------------------------------------------------- metadata

            #[test]
            fn composite_foreign_key_metadata_is_grouped() {
                let child = &<CompositeFkChild as drizzle::core::DrizzleTable>::TABLE_REF;
                assert_eq!(child.foreign_keys.len(), 1, "expected a single grouped FK");
                assert_eq!(
                    child.foreign_keys[0].source_columns,
                    &["parent_a", "parent_b"]
                );
                assert_eq!(child.foreign_keys[0].target_columns, &["id_a", "id_b"]);

                let parent = &<CompositeFkParent as drizzle::core::DrizzleTable>::TABLE_REF;
                let primary_key = parent
                    .primary_key
                    .as_ref()
                    .expect("composite primary key metadata");
                assert_eq!(primary_key.columns, &["id_a", "id_b"]);
            }

            #[test]
            fn constraint_and_relation_markers() {
                fn assert_has_pk<T: HasPrimaryKey>() {}
                fn assert_has_fk<T: HasConstraint<ForeignKeyK>>() {}
                fn assert_has_pk_constraint<T: HasConstraint<PrimaryKeyK>>() {}
                fn assert_joinable<A: Joinable<B>, B>() {}
                fn assert_relation<Child: Relation<Parent>, Parent>() {}

                assert_has_pk::<CompositeFkParent>();
                assert_has_pk_constraint::<CompositeFkParent>();
                assert_has_fk::<FkCascade>();
                assert_joinable::<FkCascade, FkParent>();

                assert_relation::<FkCascade, FkParent>();
                assert_relation::<FkBothActions, FkParent>();
                assert_relation::<CompositeFkChild, CompositeFkParent>();
                // Custom table and column names do not affect the relation.
                assert_relation::<NamedFkChild, NamedFkParent>();
            }

            // ---------------------------------------------------------- actions

            #[drizzle::test($dialect)]
            fn on_delete_cascade_removes_children(db: &mut TestDb<SharedFkSchema>) {
                let SharedFkSchema {
                    parents, cascade, ..
                } = schema;
                db.insert(parents)
                    .value(InsertFkParent::new("Parent1").with_id(1))
                    .execute();
                db.insert(cascade)
                    .value(InsertFkCascade::new("Child1").with_id(1).with_parent_id(1))
                    .execute();

                let children: Vec<SelectFkCascade> = db.select(()).from(cascade).all();
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].parent_id, Some(1));

                db.delete(parents).r#where(eq(parents.id, 1)).execute();

                let children: Vec<SelectFkCascade> = db.select(()).from(cascade).all();
                assert!(children.is_empty(), "CASCADE deletes the child");
            }

            #[drizzle::test($dialect)]
            fn on_delete_set_null_detaches_children(db: &mut TestDb<SharedFkSchema>) {
                let SharedFkSchema {
                    parents, set_null, ..
                } = schema;
                db.insert(parents)
                    .value(InsertFkParent::new("Parent1").with_id(1))
                    .execute();
                db.insert(set_null)
                    .value(InsertFkSetNull::new("Child1").with_id(1).with_parent_id(1))
                    .execute();

                db.delete(parents).r#where(eq(parents.id, 1)).execute();

                let children: Vec<SelectFkSetNull> = db.select(()).from(set_null).all();
                assert_eq!(children.len(), 1, "SET NULL keeps the child");
                assert_eq!(children[0].parent_id, None);
            }

            #[drizzle::test($dialect)]
            fn restrict_and_no_action_reject_deleting_referenced_parents(
                db: &mut TestDb<SharedFkSchema>,
            ) {
                let SharedFkSchema {
                    parents,
                    restrict,
                    no_action,
                    ..
                } = schema;
                db.insert(parents)
                    .values([
                        InsertFkParent::new("Restricted").with_id(1),
                        InsertFkParent::new("NoAction").with_id(2),
                        InsertFkParent::new("Unreferenced").with_id(3),
                    ])
                    .execute();
                db.insert(restrict)
                    .value(InsertFkRestrict::new("Child1").with_id(1).with_parent_id(1))
                    .execute();
                db.insert(no_action)
                    .value(InsertFkNoAction::new("Child2").with_id(1).with_parent_id(2))
                    .execute();

                let restricted = result!(db.delete(parents).r#where(eq(parents.id, 1)).execute());
                assert!(restricted.is_err(), "RESTRICT rejects the delete");
                let blocked = result!(db.delete(parents).r#where(eq(parents.id, 2)).execute());
                assert!(blocked.is_err(), "NO ACTION rejects the delete");

                db.delete(parents).r#where(eq(parents.id, 3)).execute();
                let remaining: Vec<SelectFkParent> = db.select(()).from(parents).all();
                assert_eq!(remaining.len(), 2);
            }

            #[drizzle::test($dialect)]
            fn on_update_cascade_and_set_null_follow_the_parent_key(
                db: &mut TestDb<SharedFkSchema>,
            ) {
                let SharedFkSchema {
                    parents,
                    update_cascade,
                    update_set_null,
                    ..
                } = schema;
                db.insert(parents)
                    .values([
                        InsertFkParent::new("Cascading").with_id(1),
                        InsertFkParent::new("Nullifying").with_id(2),
                    ])
                    .execute();
                db.insert(update_cascade)
                    .value(
                        InsertFkUpdateCascade::new("Child1")
                            .with_id(1)
                            .with_parent_id(1),
                    )
                    .execute();
                db.insert(update_set_null)
                    .value(
                        InsertFkUpdateSetNull::new("Child2")
                            .with_id(1)
                            .with_parent_id(2),
                    )
                    .execute();

                db.update(parents)
                    .set(UpdateFkParent::default().with_id(100))
                    .r#where(eq(parents.id, 1))
                    .execute();
                db.update(parents)
                    .set(UpdateFkParent::default().with_id(200))
                    .r#where(eq(parents.id, 2))
                    .execute();

                let cascaded: Vec<SelectFkUpdateCascade> = db.select(()).from(update_cascade).all();
                assert_eq!(cascaded[0].parent_id, Some(100), "ON UPDATE CASCADE");
                let nullified: Vec<SelectFkUpdateSetNull> =
                    db.select(()).from(update_set_null).all();
                assert_eq!(nullified[0].parent_id, None, "ON UPDATE SET NULL");
            }

            #[drizzle::test($dialect)]
            fn delete_and_update_actions_combine(db: &mut TestDb<SharedFkSchema>) {
                let SharedFkSchema {
                    parents,
                    both_actions,
                    ..
                } = schema;
                db.insert(parents)
                    .values([
                        InsertFkParent::new("Parent1").with_id(1),
                        InsertFkParent::new("Parent2").with_id(2),
                    ])
                    .execute();
                db.insert(both_actions)
                    .values([
                        InsertFkBothActions::new("Child1")
                            .with_id(1)
                            .with_parent_id(1),
                        InsertFkBothActions::new("Child2")
                            .with_id(2)
                            .with_parent_id(2),
                    ])
                    .execute();

                db.update(parents)
                    .set(UpdateFkParent::default().with_id(100))
                    .r#where(eq(parents.id, 1))
                    .execute();
                let child1: SelectFkBothActions = db
                    .select(())
                    .from(both_actions)
                    .r#where(eq(both_actions.value, "Child1"))
                    .get();
                assert_eq!(child1.parent_id, None, "ON UPDATE SET NULL");

                db.delete(parents).r#where(eq(parents.id, 2)).execute();
                let remaining: Vec<SelectFkBothActions> = db.select(()).from(both_actions).all();
                assert_eq!(remaining.len(), 1, "ON DELETE CASCADE removed Child2");
                assert_eq!(remaining[0].value, "Child1");
            }

            #[drizzle::test($dialect)]
            fn composite_foreign_keys_cascade_on_every_column(db: &mut TestDb<SharedFkSchema>) {
                let SharedFkSchema {
                    composite_parents,
                    composite_children,
                    ..
                } = schema;
                db.insert(composite_parents)
                    .values([
                        InsertCompositeFkParent::new("one-one")
                            .with_id_a(1)
                            .with_id_b(1),
                        InsertCompositeFkParent::new("one-two")
                            .with_id_a(1)
                            .with_id_b(2),
                    ])
                    .execute();
                db.insert(composite_children)
                    .values([
                        InsertCompositeFkChild::new("first")
                            .with_id(1)
                            .with_parent_a(1)
                            .with_parent_b(1),
                        InsertCompositeFkChild::new("second")
                            .with_id(2)
                            .with_parent_a(1)
                            .with_parent_b(2),
                    ])
                    .execute();

                db.update(composite_parents)
                    .set(UpdateCompositeFkParent::default().with_id_b(20))
                    .r#where(eq(composite_parents.id_b, 2))
                    .execute();
                let second: SelectCompositeFkChild = db
                    .select(())
                    .from(composite_children)
                    .r#where(eq(composite_children.value, "second"))
                    .get();
                assert_eq!((second.parent_a, second.parent_b), (Some(1), Some(20)));

                db.delete(composite_parents)
                    .r#where(eq(composite_parents.id_b, 1))
                    .execute();
                let remaining: Vec<SelectCompositeFkChild> =
                    db.select(()).from(composite_children).all();
                assert_eq!(remaining.len(), 1);
                assert_eq!(remaining[0].value, "second");
            }

            #[drizzle::test($dialect)]
            fn custom_column_names_still_enforce_the_reference(db: &mut TestDb<SharedFkSchema>) {
                let SharedFkSchema {
                    named_parents,
                    named_children,
                    ..
                } = schema;
                db.insert(named_parents)
                    .value(InsertNamedFkParent::new("Parent").with_id(7))
                    .execute();
                db.insert(named_children)
                    .value(InsertNamedFkChild::new().with_id(1).with_parent_id(7))
                    .execute();

                let dangling = result!(
                    db.insert(named_children)
                        .value(InsertNamedFkChild::new().with_id(2).with_parent_id(99))
                        .execute()
                );
                assert!(dangling.is_err(), "unknown parent_ref is rejected");

                let children: Vec<SelectNamedFkChild> = db.select(()).from(named_children).all();
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].parent_id, Some(7));
            }
        }
    };
}

pub(crate) use shared_foreign_key_suite;
