#[test]
fn include_migrations_embeds_v3_fixture() {
    let migrations = drizzle::include_migrations!("./tests/fixtures/include_migrations/v3");

    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].tag(), "20230331141203_init_users");
    assert_eq!(migrations[0].created_at(), 1_680_271_923_000);
    assert_eq!(migrations[0].statements().len(), 2);
    assert!(
        migrations[0].statements()[0].contains("CREATE TABLE fixture_v3_users"),
        "expected first statement to create fixture_v3_users"
    );
}
