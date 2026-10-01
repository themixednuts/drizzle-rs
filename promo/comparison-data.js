// Content for comparison.js. Every snippet here was compiled and run on
// SQLite with Rust 1.98, using the versions below; drizzle-rs snippets are
// also in verify/src/main.rs. Safety results are what SQLite did: another
// database may fail differently at runtime, but compile errors are the same.

export const VERSIONS = {
  line: "drizzle-rs 0.2.1 · diesel 2.3.9 · sea-orm 2.0.4 · toasty 0.11.0 · SQLite · Rust 1.98",
  footnote: "Every snippet compiled and run on SQLite (Oct 2026). Runtime results are SQLite's and may differ on other databases.",
};

export const SCHEMA_LINES = [
  { name: "drizzle-rs", written: 27, note: "One struct per table. Relations and insert/update models generated from <code>references</code>." },
  { name: "Diesel", written: 40, generated: 26, note: "Model structs with Insertable/AsChangeset, plus <code>table!</code> from <code>diesel print-schema</code>." },
  { name: "SeaORM", written: 49, note: "2.0 dense entities, relations as fields. <code>sea-orm-cli</code> can generate them from a database." },
  { name: "Toasty", written: 36, note: "One struct per model, relations as <code>Deferred</code> fields." },
];

export const TOPICS = [
  {
    title: "The everyday query. <span class=accent>Everyone handles it.</span>",
    question: "Users over 21, oldest first, just name and email.",
    drizzle: {
      lines: 7,
      code: `
let adults: Vec<(String, Option<String>)> = db
    .select((users.name, users.email))
    .from(users)
    .r#where(gt(users.age, 21))
    .order_by(desc(users.age))
    .limit(10)
    .all()?;`,
      note: "Reads in SQL order. The tuple is checked against the selection at compile time.",
    },
    Diesel: {
      lines: 6, status: "ran",
      code: `
let adults: Vec<(String, Option<String>)> = users::table
    .filter(users::age.gt(21))
    .order(users::age.desc())
    .limit(10)
    .select((users::name, users::email))
    .load(&mut conn)?;`,
      note: "SQL-shaped and typed end to end. Very close in spirit.",
    },
    SeaORM: {
      lines: 9, status: "ran",
      code: `
let adults: Vec<(String, Option<String>)> = User::find()
    .select_only()
    .columns([user::Column::Name, user::Column::Email])
    .filter(User::COLUMN.age.gt(21))
    .order_by_desc(User::COLUMN.age)
    .limit(10)
    .into_tuple()
    .all(&db)
    .await?;`,
      note: "Partial selects need <code>select_only</code> + <code>into_tuple</code>, and the tuple is only checked when the query runs.",
    },
    Toasty: {
      lines: 6, status: "ran",
      code: `
let adults: Vec<(String, Option<String>)> = User::filter(User::fields().age().gt(21))
    .order_by(User::fields().age().desc())
    .limit(10)
    .select((User::fields().name(), User::fields().email()))
    .exec(&mut db)
    .await?;`,
      note: "Concise, with a typed projection. Async only.",
    },
  },
  {
    title: "Aggregates. <span class=accent>Where it gets interesting.</span>",
    question: "Post count per user, only users who have posted.",
    drizzle: {
      lines: 7,
      code: `
let per_user: Vec<(String, i64)> = db
    .select((users.name, count(posts.id)))
    .from(users)
    .left_join(posts)
    .group_by(users.name)
    .having(gt(count(posts.id), 0))
    .all()?;`,
      note: "The foreign key writes the <code>ON</code> clause. <code>(String, i64)</code> is checked at compile time.",
    },
    Diesel: {
      lines: 6, status: "ran",
      code: `
let per_user: Vec<(String, i64)> = users::table
    .left_join(posts::table)
    .group_by(users::name)
    .select((users::name, count(posts::id.nullable())))
    .having(count(posts::id.nullable()).gt(0))
    .load(&mut conn)?;`,
      note: "Typed end to end. Needs <code>.nullable()</code> on the joined column and <code>joinable!</code> in the schema.",
    },
    SeaORM: {
      lines: 10, status: "ran",
      code: `
let per_user: Vec<(String, i64)> = User::find()
    .select_only()
    .column(user::Column::Name)
    .column_as(Post::COLUMN.id.count(), "posts")
    .left_join(Post)
    .group_by(user::Column::Name)
    .having(Post::COLUMN.id.count().gt(0))
    .into_tuple()
    .all(&db)
    .await?;`,
      note: "Works. A string alias, two column styles, and the tuple is decoded at runtime.",
    },
    Toasty: {
      lines: 18, status: "raw SQL",
      code: `
let rows = toasty::sql::query(
    "SELECT users.name, COUNT(posts.id) FROM users \\
     LEFT JOIN posts ON posts.author_id = users.id \\
     GROUP BY users.name HAVING COUNT(posts.id) > ?1",
)
.bind(0_i64)
.exec(&mut db)
.await?;
let per_user: Vec<(String, i64)> = rows
    .into_iter()
    .map(|row| {
        let mut cols = row.into_record().into_iter();
        Ok((
            cols.next().unwrap().try_into()?,
            cols.next().unwrap().try_into()?,
        ))
    })
    .collect::<toasty::Result<_>>()?;`,
      note: "No <code>GROUP BY</code> in the 0.11 builder. Raw SQL works, but rows come back untyped.",
    },
  },
  {
    title: "Nested data. <span class=accent>Loaded, and typed as loaded.</span>",
    question: "Every user, their posts, and each post's comments.",
    drizzle: {
      lines: 4,
      code: `
let feed = db
    .query(users)
    .with(users.posts().with(posts.comments()))
    .find_many()?;

// feed[0].posts[0].comments is a plain Vec
// without .with(..): no field \`posts\`, compile error`,
      note: "Relations you didn't load don't exist on the type, so you can't read them by mistake.",
    },
    Diesel: {
      lines: 9, status: "ran",
      code: `
let users = users::table.load::<User>(&mut conn)?;
let posts = Post::belonging_to(&users).load::<Post>(&mut conn)?;
let comments = Comment::belonging_to(&posts).load::<Comment>(&mut conn)?;
let comments_per_post = comments.grouped_by(&posts);
let posts_with_comments: Vec<(Post, Vec<Comment>)> =
    posts.into_iter().zip(comments_per_post).collect();
let posts_per_user = posts_with_comments.grouped_by(&users);
let feed: Vec<(User, Vec<(Post, Vec<Comment>)>)> =
    users.into_iter().zip(posts_per_user).collect();`,
      note: "No N+1 and fully typed, but you stitch the tree together yourself.",
    },
    SeaORM: {
      lines: 1, status: "ran",
      code: `
let feed: Vec<user::ModelEx> = User::load().with((Post, Comment)).all(&db).await?;

// skip .with(Post): user.posts.len() == 0
// user.posts[0]: panics, "HasMany is Unloaded"`,
      note: "New in 2.0, and the most concise. But an unloaded relation quietly looks empty.",
    },
    Toasty: {
      lines: 4, status: "ran",
      code: `
let feed: Vec<User> = User::all()
    .include(User::fields().posts().comments())
    .exec(&mut db)
    .await?;

// skip .include(..): u.posts.get() panics,
// "deferred field not loaded"`,
      note: "Concise. Loaded state isn't in the type unless you make the field eager (always loaded).",
    },
  },
];

const ce = (d) => ({ k: "ce", d });
const rt = (d) => ({ k: "rt", d });
const silent = (d) => ({ k: "silent", d });
const panic = (d) => ({ k: "panic", d });
const na = (d) => ({ k: "na", d });

// columns: drizzle-rs, Diesel, SeaORM, Toasty
export const SAFETY = {
  rows: [
    { label: "Compare an integer column to a string", cells: [ce(), ce(), ce("typed COLUMN API only"), ce()] },
    { label: "Filter on a table that isn't in the query", cells: [rt("no such column"), ce(), rt("no such column"), silent("matches another column")] },
    { label: "Decode into the wrong type or arity", cells: [ce(), ce(), rt("decode error"), ce()] },
    { label: "Insert without a required field", cells: [ce("new(name, age)"), rt("NOT NULL"), rt("NOT NULL"), rt("check removed in 0.8")] },
    { label: "Update that sets nothing", cells: [ce(), rt("EmptyChangeset"), silent("Ok, no-op"), panic()] },
    { label: "LEFT JOIN side decoded as non-Option", cells: [rt("unexpected NULL"), ce(), rt("null value"), na("no joins")] },
    { label: "Typo in a column or relation name", cells: [ce(), ce(), ce("string aliases unchecked"), ce()] },
    { label: "Read a relation you didn't load", cells: [ce("no such field"), na("plain Vecs"), silent("looks empty"), panic()] },
  ],
  captions: [
    [2.0, 11.6, "Diesel is strictest on query shape. Unjoined tables and non-<code>Option</code> LEFT JOINs are runtime errors in drizzle-rs."],
    [11.8, 17.4, "drizzle-rs moves required inserts, empty updates and unloaded relations to compile time."],
    [17.6, "end", "SeaORM 2.0 types its new <code>COLUMN</code> comparisons; result tuples are still checked at runtime."],
  ],
};

export const STRENGTHS = {
  others: [
    { name: "Diesel", version: "2.3", points: [
      "The strictest compile-time checking of query shape.",
      "Mature (since 2015) and widely used.",
      "PostgreSQL, MySQL, SQLite.",
      "Sync. Async through diesel-async.",
    ] },
    { name: "SeaORM", version: "2.0", points: [
      "One-line nested loading, new in 2.0.",
      "Async, plus an official sync crate.",
      "Big ecosystem: CLI codegen, migrations, Seaography, Loco.",
      "Typed column comparisons in 2.0.",
    ] },
    { name: "Toasty", version: "0.11", points: [
      "Concise models and nested <code>include()</code>.",
      "SQL databases and DynamoDB.",
      "Typed projections and comparisons.",
      "Young (pre-1.0) and moving fast.",
    ] },
  ],
  drizzle: [
    ["Two APIs, one schema", "<code>db.select()</code> for SQL-shaped work, <code>db.query()</code> for nested objects. The Drizzle ORM model."],
    ["Aggregates and nested data, both typed", "Checked <code>GROUP BY</code> tuples and relational loading. No raw SQL needed."],
    ["Compile-time checks that go wide", "Types, nullability, result shape, required inserts, non-empty updates, loaded relations."],
    ["One struct per table", "Relations and insert/update models generated from <code>references</code>."],
    ["Sync and async drivers", "rusqlite, libsql, turso, D1, postgres, tokio-postgres, mysql, mysql_async."],
    ["Migrations from your structs", "<code>drizzle generate</code> and <code>migrate</code>, or apply them at startup."],
  ],
};
