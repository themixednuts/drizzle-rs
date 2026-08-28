//! Driver-neutral MySQL query builder acceptance tests.

use drizzle::core::expr::{
    Expr, NonNull, Null, alias, and, avg, char_length, concat, count, eq, gt, length, octet_length,
    sum, variance,
};
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

type MySQLDecimal = <drizzle::mysql::types::Decimal as drizzle::core::SQLTypeToRust<
    drizzle::mysql::MySQLDialect,
>>::RustType;
type MySQLArithmeticRow = (u64, Option<MySQLDecimal>, Option<u64>, i64);
type MySQLAggregateRow = (
    Option<MySQLDecimal>,
    Option<MySQLDecimal>,
    Option<f64>,
    i64,
    i64,
    i64,
);

#[MySQLTable(NAME = "users")]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
    #[column(DEFAULT = true)]
    active: bool,
}

#[MySQLTable(NAME = "posts")]
struct Posts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(REFERENCES = Users::id)]
    user_id: u64,
    title: String,
}

#[MySQLTable(NAME = "defaults_only")]
struct DefaultsOnly {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
}

#[MySQLTable(NAME = "generated_posts")]
struct GeneratedPosts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    title: String,
    #[column(generated(STORED, "CHAR_LENGTH(title)"))]
    title_len: u32,
}

#[MySQLIndex]
struct UsersNameIdx(Users::name);

#[MySQLIndex]
struct UsersActiveIdx(Users::active);

#[MySQLIndex]
struct PostsUserIdIdx(Posts::user_id);

#[MySQLIndex]
struct PostsIdIdx(Posts::id);

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    posts: Posts,
}

#[derive(MySQLSchema)]
struct DefaultsSchema {
    defaults_only: DefaultsOnly,
}

#[derive(MySQLSchema)]
struct GeneratedSchema {
    generated_posts: GeneratedPosts,
}

fn builder() -> QueryBuilder<'static, Schema, drizzle::mysql::builder::BuilderInit> {
    QueryBuilder::new::<Schema>()
}

fn generated_builder()
-> QueryBuilder<'static, GeneratedSchema, drizzle::mysql::builder::BuilderInit> {
    QueryBuilder::new::<GeneratedSchema>()
}

#[test]
fn select_uses_the_shared_public_grammar() {
    let Schema { users, posts } = Schema::new();
    let query = builder()
        .select_distinct((users.id, alias(concat(users.name, "!"), "label")))
        .from(users)
        .inner_join((posts, eq(posts.user_id, users.id)))
        .r#where(eq(users.active, true))
        .group_by((users.id, users.name))
        .having(gt(count(users.id), 0))
        .order_by(desc(users.id))
        .limit(10)
        .offset(2);

    assert_eq!(
        query.to_sql().sql(),
        "SELECT DISTINCT `users`.`id`, CONCAT(`users`.`name`, ?) AS `label` FROM `users` INNER JOIN `posts` ON `posts`.`user_id` = `users`.`id` WHERE `users`.`active` = ? GROUP BY `users`.`id`, `users`.`name` HAVING COUNT(`users`.`id`)> ? ORDER BY `users`.`id` DESC LIMIT ? OFFSET ?"
    );
}

#[test]
fn select_all_is_inferred_from_from_without_a_new_verb() {
    let Schema { users, .. } = Schema::new();
    let query = builder().select(()).from(users);
    fn assert_users_row<State, Table, Marker, Grouped>(
        _: &QueryBuilder<'_, Schema, State, Table, Marker, SelectUsers, Grouped>,
    ) {
    }
    assert_users_row(&query);
    assert_eq!(
        query.to_sql().sql(),
        "SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users`"
    );
}

#[test]
fn standalone_offset_is_valid_mysql() {
    let Schema { users, .. } = Schema::new();
    let query = builder()
        .select(users.id)
        .from(users)
        .r#where(eq(users.active, true))
        .order_by(asc(users.id))
        .offset(5);

    assert_eq!(
        query.to_sql().sql(),
        "SELECT `users`.`id` FROM `users` WHERE `users`.`active` = ? ORDER BY `users`.`id` ASC LIMIT 18446744073709551615 OFFSET ?"
    );
}

#[test]
fn set_operations_parenthesize_operands_and_unqualify_global_order() {
    let Schema { users, .. } = Schema::new();
    let left = builder()
        .select(users.name)
        .from(users)
        .order_by(asc(users.name))
        .limit(2);
    let right = builder().select(users.name).from(users);
    let query = left.union_all(right).order_by(desc(users.name)).limit(3);

    assert_eq!(
        query.to_sql().sql(),
        "(SELECT `users`.`name` FROM `users` ORDER BY `users`.`name` ASC LIMIT ?) UNION ALL (SELECT `users`.`name` FROM `users`) ORDER BY `name` DESC LIMIT ?"
    );
}

#[test]
fn set_operations_match_rows_across_tables_and_order_by_output_alias() {
    let Schema { users, posts } = Schema::new();
    let ids = builder()
        .select(users.id)
        .from(users)
        .union(builder().select(posts.user_id).from(posts));
    assert_eq!(
        ids.to_sql().sql(),
        "(SELECT `users`.`id` FROM `users`) UNION (SELECT `posts`.`user_id` FROM `posts`)"
    );

    let labels = builder()
        .select(alias(users.name, "label"))
        .from(users)
        .union(builder().select(alias(posts.title, "label")).from(posts))
        .order_by(desc(output_alias("label")));
    assert_eq!(
        labels.to_sql().sql(),
        "(SELECT `users`.`name` AS `label` FROM `users`) UNION (SELECT `posts`.`title` AS `label` FROM `posts`) ORDER BY `label` DESC"
    );
}

#[test]
fn mysql_8031_set_operation_surface_is_complete() {
    let Schema { users, .. } = Schema::new();
    let query = || builder().select(users.id).from(users);
    let operand = "(SELECT `users`.`id` FROM `users`)";

    assert_eq!(
        query().union(query()).to_sql().sql(),
        format!("{operand} UNION {operand}")
    );
    assert_eq!(
        query().union_all(query()).to_sql().sql(),
        format!("{operand} UNION ALL {operand}")
    );
    assert_eq!(
        query().intersect(query()).to_sql().sql(),
        format!("{operand} INTERSECT {operand}")
    );
    assert_eq!(
        query().intersect_all(query()).to_sql().sql(),
        format!("{operand} INTERSECT ALL {operand}")
    );
    assert_eq!(
        query().except(query()).to_sql().sql(),
        format!("{operand} EXCEPT {operand}")
    );
    assert_eq!(
        query().except_all(query()).to_sql().sql(),
        format!("{operand} EXCEPT ALL {operand}")
    );
    assert_eq!(
        query().union(query()).offset(3).to_sql().sql(),
        format!("{operand} UNION {operand} LIMIT 18446744073709551615 OFFSET ?")
    );
}

#[test]
fn joins_render_only_mysql_supported_kinds() {
    let Schema { users, posts } = Schema::new();
    let condition = || eq(posts.user_id, users.id);
    let sql =
        |query: QueryBuilder<'_, Schema, drizzle::mysql::builder::SelectJoinSet, Posts, _, _>| {
            query.to_sql().sql()
        };

    assert_eq!(
        builder()
            .select(())
            .from(users)
            .inner_join((posts, condition()))
            .to_sql()
            .sql(),
        "SELECT `users`.`id`, `users`.`name`, `users`.`active`, `posts`.`id`, `posts`.`user_id`, `posts`.`title` FROM `users` INNER JOIN `posts` ON `posts`.`user_id` = `users`.`id`"
    );

    assert!(
        sql(builder()
            .select(users.id)
            .from(users)
            .join((posts, condition())))
        .contains(" JOIN `posts` ON ")
    );
    assert!(
        sql(builder()
            .select(users.id)
            .from(users)
            .inner_join((posts, condition())))
        .contains(" INNER JOIN `posts` ON ")
    );
    assert!(
        sql(builder()
            .select(users.id)
            .from(users)
            .cross_join((posts, condition())))
        .contains(" INNER JOIN `posts` ON ")
    );
    assert!(
        builder()
            .select(users.id)
            .from(users)
            .left_join((posts, condition()))
            .to_sql()
            .sql()
            .contains(" LEFT JOIN `posts` ON ")
    );
    assert!(
        builder()
            .select(users.id)
            .from(users)
            .left_outer_join((posts, condition()))
            .to_sql()
            .sql()
            .contains(" LEFT OUTER JOIN `posts` ON ")
    );
    assert!(
        sql(builder()
            .select(users.id)
            .from(users)
            .right_join((posts, condition())))
        .contains(" RIGHT JOIN `posts` ON ")
    );
    assert!(
        sql(builder()
            .select(users.id)
            .from(users)
            .right_outer_join((posts, condition())))
        .contains(" RIGHT OUTER JOIN `posts` ON ")
    );
}

#[test]
fn derived_sources_support_lateral_joins() {
    struct UserPosts;
    impl drizzle::core::Tag for UserPosts {
        const NAME: &'static str = "user_posts";
    }

    let Schema { users, posts } = Schema::new();
    let posts_for_user = || {
        builder()
            .select((posts.user_id, posts.title))
            .from(posts)
            .r#where(eq(posts.user_id, users.id))
            .alias(UserPosts)
    };

    let source = posts_for_user();
    let (post_user_id, title) = source.fields();
    let inner = builder()
        .select((users.id, title))
        .from(users)
        .inner_join_lateral((source, eq(post_user_id, users.id)));
    assert_eq!(
        inner.to_sql().sql(),
        "SELECT `users`.`id`, `user_posts`.`title` FROM `users` INNER JOIN LATERAL (SELECT `posts`.`user_id`, `posts`.`title` FROM `posts` WHERE `posts`.`user_id` = `users`.`id`) AS `user_posts` ON `user_posts`.`user_id` = `users`.`id`"
    );

    let source = posts_for_user();
    let (post_user_id, _) = source.fields();
    let all = builder()
        .select(())
        .from(users)
        .inner_join_lateral((source, eq(post_user_id, users.id)));
    assert_eq!(
        all.to_sql().sql(),
        "SELECT `users`.`id`, `users`.`name`, `users`.`active`, `user_posts`.* FROM `users` INNER JOIN LATERAL (SELECT `posts`.`user_id`, `posts`.`title` FROM `posts` WHERE `posts`.`user_id` = `users`.`id`) AS `user_posts` ON `user_posts`.`user_id` = `users`.`id`"
    );

    let source = posts_for_user();
    let (_, title) = source.fields();
    let cross = builder()
        .select((users.id, title))
        .from(users)
        .cross_join_lateral(source);
    assert_eq!(
        cross.to_sql().sql(),
        "SELECT `users`.`id`, `user_posts`.`title` FROM `users` CROSS JOIN LATERAL (SELECT `posts`.`user_id`, `posts`.`title` FROM `posts` WHERE `posts`.`user_id` = `users`.`id`) AS `user_posts`"
    );
}

#[test]
fn arithmetic_uses_mysql_result_types_nullability_and_row_inference() {
    let Schema { users, .. } = Schema::new();
    let added = users.id + users.id;
    let divided = users.id / users.id;
    let remainder = users.id % users.id;
    let negated = -users.id;

    fn assert_expr_type<E, SQLType, Nullable>(_: &E)
    where
        E: Expr<
                'static,
                drizzle::mysql::MySQLValue<'static>,
                SQLType = SQLType,
                Nullable = Nullable,
            >,
        SQLType: drizzle::core::types::DataType,
        Nullable: drizzle::core::expr::Nullability,
    {
    }

    assert_expr_type::<_, drizzle::mysql::types::BigIntUnsigned, NonNull>(&added);
    assert_expr_type::<_, drizzle::mysql::types::Decimal, Null>(&divided);
    assert_expr_type::<_, drizzle::mysql::types::BigIntUnsigned, Null>(&remainder);
    assert_expr_type::<_, drizzle::mysql::types::BigInt, NonNull>(&negated);

    let query = builder()
        .select((added, divided, remainder, negated))
        .from(users);
    fn assert_row<State, Table, Marker, Grouped>(
        _: &QueryBuilder<'_, Schema, State, Table, Marker, MySQLArithmeticRow, Grouped>,
    ) {
    }
    assert_row(&query);
    assert_eq!(
        query.to_sql().sql(),
        "SELECT `users`.`id` + `users`.`id`, `users`.`id` / `users`.`id`, `users`.`id` % `users`.`id`, - `users`.`id` FROM `users`"
    );
}

#[test]
fn mysql_aggregate_and_length_policies_match_server_results() {
    let Schema { users, .. } = Schema::new();
    let query = builder()
        .select((
            sum(users.id),
            avg(users.id),
            variance(users.id),
            length(users.name),
            char_length(users.name),
            octet_length(users.name),
        ))
        .from(users);

    fn assert_row<State, Table, Marker, Grouped>(
        _: &QueryBuilder<'_, Schema, State, Table, Marker, MySQLAggregateRow, Grouped>,
    ) {
    }
    assert_row(&query);
    assert_eq!(
        query.to_sql().sql(),
        "SELECT SUM(`users`.`id`), AVG(`users`.`id`), VAR_SAMP(`users`.`id`), LENGTH(`users`.`name`), CHAR_LENGTH(`users`.`name`), OCTET_LENGTH(`users`.`name`) FROM `users`"
    );
}

#[test]
fn ctes_prefix_select_update_and_delete() {
    struct ActiveUsers;
    impl drizzle::core::Tag for ActiveUsers {
        const NAME: &'static str = "active_users";
    }

    let Schema { users, .. } = Schema::new();
    let cte = builder()
        .select((users.id, users.name, users.active))
        .from(users)
        .r#where(eq(users.active, true))
        .into_cte::<ActiveUsers>();
    let query = builder().with(&cte).select((cte.id, cte.name)).from(&cte);

    assert_eq!(
        query.to_sql().sql(),
        "WITH `active_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` WHERE `users`.`active` = ?) SELECT `active_users`.`id`, `active_users`.`name` FROM `active_users`"
    );

    struct AllUsers;
    impl drizzle::core::Tag for AllUsers {
        const NAME: &'static str = "all_users";
    }
    let second = builder()
        .select((users.id, users.name, users.active))
        .from(users)
        .into_cte::<AllUsers>();
    let multiple = builder().with(&cte).with(&second).select(cte.id).from(&cte);
    assert_eq!(
        multiple.to_sql().sql(),
        "WITH `active_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` WHERE `users`.`active` = ?), `all_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users`) SELECT `active_users`.`id` FROM `active_users`"
    );

    let update = builder()
        .with(&cte)
        .update(users)
        .set(UpdateUsers::default().with_name("updated"))
        .r#where(eq(users.id, 1_u64));
    assert_eq!(
        update.to_sql().sql(),
        "WITH `active_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` WHERE `users`.`active` = ?) UPDATE `users` SET `name` = ? WHERE `users`.`id` = ?"
    );

    let delete = builder()
        .with(&cte)
        .delete(users)
        .r#where(eq(users.id, 1_u64));
    assert_eq!(
        delete.to_sql().sql(),
        "WITH `active_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` WHERE `users`.`active` = ?) DELETE FROM `users` WHERE `users`.`id` = ?"
    );

    let insert = builder()
        .insert(users)
        .columns((users.id, users.name, users.active))
        .select_raw(
            builder()
                .with(&cte)
                .select((cte.id, cte.name, cte.active))
                .from(&cte),
        );
    assert_eq!(
        insert.to_sql().sql(),
        "INSERT INTO `users` (`id`, `name`, `active`) WITH `active_users` AS (SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` WHERE `users`.`active` = ?) SELECT `active_users`.`id`, `active_users`.`name`, `active_users`.`active` FROM `active_users`"
    );
}

#[test]
fn insert_update_and_delete_follow_mysql_clause_order() {
    let Schema { users, .. } = Schema::new();

    let insert = builder().insert(users).value(InsertUsers::new("Alice"));
    assert_eq!(
        insert.to_sql().sql(),
        "INSERT INTO `users` (`name`) VALUES (?)"
    );
    let many = builder()
        .insert(users)
        .values([InsertUsers::new("Alice"), InsertUsers::new("Bob")]);
    assert_eq!(
        many.to_sql().sql(),
        "INSERT INTO `users` (`name`) VALUES (?), (?)"
    );

    let defaults_builder = QueryBuilder::new::<DefaultsSchema>();
    let DefaultsSchema { defaults_only } = DefaultsSchema::new();
    let defaults = defaults_builder
        .insert(defaults_only)
        .value(InsertDefaultsOnly::new());
    assert_eq!(
        defaults.to_sql().sql(),
        "INSERT INTO `defaults_only` () VALUES ()"
    );

    let selected = builder()
        .select((users.id, users.name, users.active))
        .from(users);
    let insert_selected = builder().insert(users).select(selected);
    assert_eq!(
        insert_selected.to_sql().sql(),
        "INSERT INTO `users` (`id`, `name`, `active`) SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users`"
    );

    let Schema { users, posts } = Schema::new();
    let partial_insert_selected = builder()
        .insert(posts)
        .columns((posts.user_id, posts.title))
        .select(builder().select((users.id, users.name)).from(users));
    assert_eq!(
        partial_insert_selected.to_sql().sql(),
        "INSERT INTO `posts` (`user_id`, `title`) SELECT `users`.`id`, `users`.`name` FROM `users`"
    );

    let borrowed_insert_selected = builder()
        .insert(&posts)
        .columns((posts.user_id, posts.title))
        .select(builder().select((users.id, users.name)).from(users));
    assert_eq!(
        borrowed_insert_selected.to_sql().sql(),
        "INSERT INTO `posts` (`user_id`, `title`) SELECT `users`.`id`, `users`.`name` FROM `users`"
    );

    let GeneratedSchema { generated_posts } = GeneratedSchema::new();
    let single_column_insert_selected = generated_builder()
        .insert(generated_posts)
        .columns(generated_posts.title)
        .select(
            generated_builder()
                .select(generated_posts.title)
                .from(generated_posts),
        );
    assert_eq!(
        single_column_insert_selected.to_sql().sql(),
        "INSERT INTO `generated_posts` (`title`) SELECT `generated_posts`.`title` FROM `generated_posts`"
    );

    let GeneratedSchema { generated_posts } = GeneratedSchema::new();
    let generated_insert_selected = generated_builder().insert(generated_posts).select(
        generated_builder()
            .select((generated_posts.id, generated_posts.title))
            .from(generated_posts),
    );
    assert_eq!(
        generated_insert_selected.to_sql().sql(),
        "INSERT INTO `generated_posts` (`id`, `title`) SELECT `generated_posts`.`id`, `generated_posts`.`title` FROM `generated_posts`"
    );

    let Schema { users, posts } = Schema::new();
    let ignored_partial_insert_selected = builder()
        .insert(posts)
        .ignore()
        .columns((posts.user_id, posts.title))
        .select(builder().select((users.id, users.name)).from(users));
    assert_eq!(
        ignored_partial_insert_selected.to_sql().sql(),
        "INSERT IGNORE INTO `posts` (`user_id`, `title`) SELECT `users`.`id`, `users`.`name` FROM `users`"
    );

    let update = builder()
        .update(users)
        .set(UpdateUsers::default().with_name("Bob"))
        .r#where(eq(users.active, true))
        .order_by(desc(users.id))
        .limit(1);
    assert_eq!(
        update.to_sql().sql(),
        "UPDATE `users` SET `name` = ? WHERE `users`.`active` = ? ORDER BY `users`.`id` DESC LIMIT ?"
    );

    let delete = builder()
        .delete(users)
        .r#where(eq(users.active, false))
        .order_by(asc(users.id))
        .limit(4);
    assert_eq!(
        delete.to_sql().sql(),
        "DELETE FROM `users` WHERE `users`.`active` = ? ORDER BY `users`.`id` ASC LIMIT ?"
    );
}

#[test]
#[should_panic(expected = "an INSERT target column cannot appear more than once")]
fn insert_select_rejects_duplicate_target_columns() {
    let GeneratedSchema { generated_posts } = GeneratedSchema::new();
    let _ = generated_builder()
        .insert(generated_posts)
        .columns((generated_posts.title, generated_posts.title));
}

#[test]
fn native_insert_conflict_forms_preserve_mysql_semantics() {
    let Schema { users, .. } = Schema::new();

    let ignored = builder()
        .insert(users)
        .ignore()
        .value(InsertUsers::new("Alice"));
    assert_eq!(
        ignored.to_sql().sql(),
        "INSERT IGNORE INTO `users` (`name`) VALUES (?)"
    );

    let upsert = builder()
        .insert(users)
        .value(InsertUsers::new("Alice"))
        .on_duplicate_key_update(UpdateUsers::default().with_name("updated"));
    assert_eq!(
        upsert.to_sql().sql(),
        "INSERT INTO `users` (`name`) VALUES (?) ON DUPLICATE KEY UPDATE `name` = ?"
    );
    assert_eq!(upsert.prepare().params.len(), 2);
}

#[test]
fn select_index_hints_are_tied_to_their_generated_table_metadata() {
    let Schema { users, posts } = Schema::new();
    let base = builder()
        .select(users.id)
        .from(users)
        .use_index(UsersNameIdx::new());
    assert_eq!(
        base.to_sql().sql(),
        "SELECT `users`.`id` FROM `users` USE INDEX (`users_name_idx`)"
    );

    let multiple = builder()
        .select(users.id)
        .from(users)
        .use_index((UsersNameIdx::new(), UsersActiveIdx::new()));
    assert_eq!(
        multiple.to_sql().sql(),
        "SELECT `users`.`id` FROM `users` USE INDEX (`users_name_idx`, `users_active_idx`)"
    );

    let forced = builder()
        .select(users.id)
        .from(users)
        .force_index(UsersNameIdx::new());
    assert_eq!(
        forced.to_sql().sql(),
        "SELECT `users`.`id` FROM `users` FORCE INDEX (`users_name_idx`)"
    );

    let ignored = builder()
        .select(users.id)
        .from(users)
        .ignore_index(UsersNameIdx::new());
    assert_eq!(
        ignored.to_sql().sql(),
        "SELECT `users`.`id` FROM `users` IGNORE INDEX (`users_name_idx`)"
    );

    let hinted_union = builder()
        .select(users.id)
        .from(users)
        .use_index(UsersNameIdx::new())
        .union(builder().select(users.id).from(users));
    assert_eq!(
        hinted_union.to_sql().sql(),
        "(SELECT `users`.`id` FROM `users` USE INDEX (`users_name_idx`)) UNION (SELECT `users`.`id` FROM `users`)"
    );

    let hinted_insert = builder().insert(users).select(
        builder()
            .select((users.id, users.name, users.active))
            .from(users)
            .use_index(UsersNameIdx::new()),
    );
    assert_eq!(
        hinted_insert.to_sql().sql(),
        "INSERT INTO `users` (`id`, `name`, `active`) SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users` USE INDEX (`users_name_idx`)"
    );

    let explicit_join = builder()
        .select((users.id, posts.id))
        .from(users)
        .inner_join((
            posts.use_index((PostsUserIdIdx::new(), PostsIdIdx::new())),
            eq(posts.user_id, users.id),
        ));
    assert_eq!(
        explicit_join.to_sql().sql(),
        "SELECT `users`.`id`, `posts`.`id` FROM `users` INNER JOIN `posts` USE INDEX (`posts_user_id_idx`, `posts_id_idx`) ON `posts`.`user_id` = `users`.`id`"
    );

    let automatic_join = builder()
        .select((users.id, posts.id))
        .from(users)
        .inner_join(posts.force_index(PostsUserIdIdx::new()));
    assert_eq!(
        automatic_join.to_sql().sql(),
        "SELECT `users`.`id`, `posts`.`id` FROM `users` INNER JOIN `posts` FORCE INDEX (`posts_user_id_idx`) ON `posts`.`user_id` = `users`.`id`"
    );
}

#[test]
fn locking_reads_offer_only_mysql_strengths_and_one_wait_policy() {
    let Schema { users, .. } = Schema::new();

    assert_eq!(
        builder()
            .select(users.id)
            .from(users)
            .for_update()
            .to_sql()
            .sql(),
        "SELECT `users`.`id` FROM `users` FOR UPDATE"
    );
    assert_eq!(
        builder()
            .select(users.id)
            .from(users)
            .for_share()
            .nowait()
            .to_sql()
            .sql(),
        "SELECT `users`.`id` FROM `users` FOR SHARE NOWAIT"
    );
    assert_eq!(
        builder()
            .select(users.id)
            .from(users)
            .limit(2)
            .for_update()
            .skip_locked()
            .to_sql()
            .sql(),
        "SELECT `users`.`id` FROM `users` LIMIT ? FOR UPDATE SKIP LOCKED"
    );
}

#[test]
fn mysql_mutation_results_expose_ok_packet_metadata() {
    let result = MySQLMutationResult::new(3, Some(41));
    assert_eq!(result.affected_rows(), 3);
    assert_eq!(result.last_insert_id(), Some(41));
    assert_eq!(MySQLMutationResult::default().last_insert_id(), None);
}

#[test]
#[should_panic(expected = "insert values requires at least one row")]
fn empty_batch_insert_is_rejected_at_the_builder_boundary() {
    let Schema { users, .. } = Schema::new();
    let values: [InsertUsers<'static>; 0] = [];
    let _ = builder().insert(users).values(values);
}

#[test]
fn prepare_orders_named_placeholders_for_mysql_positional_binding() {
    let Schema { users, .. } = Schema::new();
    let query =
        builder().select(users.id).from(users).r#where(
            eq(
                users.id,
                TypedPlaceholder::<
                    drizzle::mysql::types::BigIntUnsigned,
                    drizzle::core::expr::NonNull,
                >::named("user_id"),
            ),
        );
    let prepared = query.prepare();

    assert_eq!(
        prepared.sql(),
        "SELECT `users`.`id` FROM `users` WHERE `users`.`id` = ?"
    );
    assert_eq!(prepared.external_param_count(), 1);
    assert_eq!(prepared.params[0].placeholder.name, Some("user_id"));
}

#[test]
fn prepare_is_available_for_insert_update_and_delete() {
    let Schema { users, .. } = Schema::new();
    let name = || {
        TypedPlaceholder::<drizzle::mysql::types::Text, drizzle::core::expr::NonNull>::named(
            "user_name",
        )
    };
    let id = || {
        TypedPlaceholder::<
            drizzle::mysql::types::BigIntUnsigned,
            drizzle::core::expr::NonNull,
        >::named("user_id")
    };

    let insert = builder()
        .insert(users)
        .value(InsertUsers::new(name()))
        .prepare();
    assert_eq!(insert.sql(), "INSERT INTO `users` (`name`) VALUES (?)");
    assert_eq!(insert.external_param_count(), 1);
    assert_eq!(insert.params[0].placeholder.name, Some("user_name"));

    let update = builder()
        .update(users)
        .set(UpdateUsers::default().with_name(name()))
        .r#where(eq(users.id, id()))
        .prepare();
    assert_eq!(
        update.sql(),
        "UPDATE `users` SET `name` = ? WHERE `users`.`id` = ?"
    );
    assert_eq!(update.external_param_count(), 2);
    assert_eq!(
        update
            .params
            .iter()
            .map(|param| param.placeholder.name)
            .collect::<Vec<_>>(),
        vec![Some("user_name"), Some("user_id")]
    );

    let delete = builder()
        .delete(users)
        .r#where(eq(users.id, id()))
        .prepare();
    assert_eq!(delete.sql(), "DELETE FROM `users` WHERE `users`.`id` = ?");
    assert_eq!(delete.external_param_count(), 1);
    assert_eq!(delete.params[0].placeholder.name, Some("user_id"));
}

#[test]
fn named_bindings_are_reordered_and_repeated_for_the_wire_plan() {
    let Schema { users, .. } = Schema::new();
    let user_id = || {
        TypedPlaceholder::<
            drizzle::mysql::types::BigIntUnsigned,
            drizzle::core::expr::NonNull,
        >::named("user_id")
    };
    let user_name =
        TypedPlaceholder::<drizzle::mysql::types::Text, drizzle::core::expr::NonNull>::named(
            "user_name",
        );
    let query = builder().select(users.id).from(users).r#where(and(
        eq(users.id, user_id()),
        and(eq(users.name, user_name), eq(users.id, user_id())),
    ));
    let prepared = query.prepare();
    assert_eq!(
        prepared.sql(),
        "SELECT `users`.`id` FROM `users` WHERE (`users`.`id` = ? AND (`users`.`name` = ? AND `users`.`id` = ?))"
    );
    assert_eq!(prepared.external_param_count(), 2);
    let (_, values) = prepared
        .bind([
            ParamBind::new("user_name", MySQLValue::from("Alice")),
            ParamBind::new("user_id", MySQLValue::UInt(7)),
        ])
        .expect("all named bindings are present");

    assert_eq!(
        values.collect::<Vec<_>>(),
        vec![
            MySQLValue::UInt(7),
            MySQLValue::from("Alice"),
            MySQLValue::UInt(7),
        ]
    );
}
