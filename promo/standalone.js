// "drizzle-rs" — standalone promo. Every snippet compiles: see verify/src/main.rs.
import { Timeline, boot, h, prog, ease, reveal, lerp, frame, Code } from "./engine/engine.js";
import { stage } from "./engine/stage.js";
import { at, header, captions, codeWindow, Boxes, stagger } from "./engine/kit.js";

const root = document.getElementById("stage");
const tl = new Timeline(root);
const CH = ["Schema", "db.select()", "db.query()", "Which one?", "Type safety", "Ship it"];

// ─────────────────────────────────────────────────────────────── 1. intro
tl.scene(6, (el) => {
  const logo = at(el, 0, 330, h("div", { class: "logo", style: { fontSize: "190px", width: "1920px", justifyContent: "center" } },
    h("span", {}, "drizzle"), h("span", { class: "rs" }, "-rs")));
  const tag = at(el, 0, 590, h("div", { class: "headline", style: { width: "1920px", textAlign: "center", fontSize: "60px" }, html: "Type-safe SQL for Rust." }));
  const sub = at(el, 0, 690, h("div", { class: "sub", style: { width: "1920px", textAlign: "center", fontSize: "34px" }, html: "Write it like SQL. Get it checked like Rust." }));
  const ver = at(el, 0, 800, h("div", { style: { width: "1920px", display: "flex", justifyContent: "center", gap: "16px" } },
    ["SQLite", "PostgreSQL", "MySQL"].map((d) => h("span", { class: "pill" }, d))));
  return (lt, len) => {
    const p = prog(lt, 0.2, 1.4, ease.out);
    logo.style.opacity = p * (1 - prog(lt, len - 0.6, len));
    logo.style.transform = `translateY(${(1 - p) * 40}px) scale(${0.94 + 0.06 * p})`;
    logo.style.letterSpacing = `${lerp(0.04, -0.05, p)}em`;
    reveal(tag, lt, 1.3, len - 0.6);
    reveal(sub, lt, 2.0, len - 0.6);
    stagger([...ver.children], lt, 2.8, 0.15, len - 0.6);
  };
});

// ─────────────────────────────────────────────────────────────── 2. schema
const SCHEMA_A = `
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: i64,
}`;
const SCHEMA_B = `
#[SQLiteTable]
pub struct Users {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
    pub email: «Option<String>»,
    pub age: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    #[column(«references = Users::id»)]
    pub author_id: i64,
}`;
const SCHEMA_C = `
#[SQLiteTable] pub struct Users { ⋯ }
#[SQLiteTable] pub struct Posts { ⋯ }

#[SQLiteTable]
pub struct Comments {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub body: String,
    #[column(references = Posts::id)]
    pub post_id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
    pub comments: Comments,
}`;
tl.scene(17, (el, len) => {
  const head = header(el, "01 · Schema", "Your schema is <span class=accent>just Rust.</span>");
  const w = codeWindow(el, 110, 260, {
    title: "src/schema.rs", width: 900, size: 20, lh: 1.4, height: 572, typeSpeed: 48,
    states: [{ at: 0.8, code: SCHEMA_A, type: true }, { at: 6.0, code: SCHEMA_B }, { at: 11.0, code: SCHEMA_C }],
  });
  // right column: what the macro generates
  const rx = 1080;
  const gen = at(el, rx, 268, h("div", { class: "kicker", style: { color: "var(--muted)" } }, "The macro writes the rest"));
  const models = [
    ["SelectUsers", "rows you read"],
    ["InsertUsers", "new(name, age) · .with_email(…)"],
    ["UpdateUsers", "only the columns you set"],
    ["PartialSelectUsers", "pick-your-columns results"],
  ].map(([n, d], i) =>
    at(el, rx, 320 + i * 66, h("div", { style: { display: "flex", gap: "18px", alignItems: "center" } },
      h("span", { class: "chip" }, n), h("span", { class: "sub", style: { fontSize: "22px" } }, d))));
  const rel = at(el, rx, 600, h("div", { style: { display: "flex", gap: "14px", alignItems: "center" } },
    h("span", { class: "chip", style: { color: "var(--s-fn)" } }, "users.posts()"),
    h("span", { class: "chip", style: { color: "var(--s-fn)" } }, "posts.author()"),
    h("span", { class: "sub", style: { fontSize: "22px" } }, "from references")));
  const ddl = codeWindow(el, rx, 680, {
    title: "drizzle/…_init/migration.sql", width: 730, size: 17, lang: "sql", kind: "sql",
    states: [{ at: 0, code: `
CREATE TABLE \`posts\` (
  \`id\` INTEGER PRIMARY KEY AUTOINCREMENT,
  \`title\` TEXT NOT NULL,
  \`content\` TEXT,
  \`author_id\` INTEGER NOT NULL,
  CONSTRAINT \`posts_author_id_fkey\` FOREIGN KEY …
);` }],
  });
  const cap = captions(el, 110, 900, [
    [6.4, 9.0, "<code>Option&lt;T&gt;</code> means nullable. Plain types mean <code>NOT NULL</code>."],
    [9.0, 11.0, "<code>references</code> gives you a foreign key <em>and</em> relations."],
    [11.4, len, "One struct per table. Migrations, models and relations: generated."],
  ], { width: 960 });
  return (lt) => {
    head(lt, len);
    reveal(w.win, lt, 0.3, len - 0.5);
    w.update(lt);
    reveal(gen, lt, 3.6, len - 0.5);
    stagger(models, lt, 3.8, 0.25, len - 0.5);
    reveal(rel, lt, 9.2, len - 0.5);
    reveal(ddl.win, lt, 11.6, len - 0.5);
    cap(lt);
  };
}, { chapter: "Schema" });

// ─────────────────────────────────────────────────────────────── 3. connect
tl.scene(6.5, (el, len) => {
  const head = header(el, "01 · Schema", "Tables are <span class=accent>values.</span> Destructure and go.");
  const w = codeWindow(el, 110, 330, {
    title: "src/main.rs", width: 1700, size: 30, typeSpeed: 60,
    states: [{ at: 0.6, type: true, code: `
let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts, comments }) = Drizzle::new(conn);` }],
  });
  const pills = at(el, 110, 560, h("div", { style: { display: "flex", gap: "16px" } },
    ["You own the connection", "Sync or async drivers", "No global state", "No runtime schema registry"].map((t) => h("span", { class: "pill" }, t))));
  return (lt) => {
    head(lt, len);
    reveal(w.win, lt, 0.3, len - 0.5);
    w.update(lt);
    stagger([...pills.children], lt, 3.2, 0.18, len - 0.5);
  };
}, { chapter: "Schema" });

// ─────────────────────────────────────────────────────────────── 4. two ways
tl.scene(7, (el, len) => {
  const title = at(el, 0, 140, h("div", { class: "headline", style: { width: "1920px", textAlign: "center" }, html: "Two ways to ask. <span class=accent>One schema.</span>" }));
  const sub = at(el, 0, 240, h("div", { class: "sub", style: { width: "1920px", textAlign: "center" }, html: "The same split as Drizzle ORM in TypeScript, checked by rustc." }));
  const card = (x, api, name, line, code) => {
    const c = at(el, x, 360, h("div", { class: "card", style: { width: "820px", height: "470px" } }));
    c.append(
      h("div", { class: "mono", style: { fontSize: "54px", fontWeight: 700, color: "var(--lime)" } }, api),
      h("div", { class: "kicker", style: { color: "var(--muted)", marginTop: "18px" } }, name),
      h("div", { class: "caption", style: { marginTop: "26px", fontSize: "34px", lineHeight: 1.3 } }, line),
    );
    const k = new Code({ size: 23, states: [{ at: 0, code }] });
    k.el.style.marginTop = "40px";
    c.append(k.el);
    k.update(0);
    return c;
  };
  const a = card(110, "db.select()", "SQL-like", "If you know SQL, you already know this API.", `
db.select((users.name, users.email))
    .from(users)
    .r#where(gt(users.age, 21))`);
  const b = card(990, "db.query()", "Relational", "Ask for objects. Get nested, typed data back.", `
db.query(users)
    .with(users.posts())
    .find_many()`);
  return (lt) => {
    reveal(title, lt, 0.1, len - 0.5);
    reveal(sub, lt, 0.5, len - 0.5);
    reveal(a, lt, 1.2, len - 0.5, 0.6, 40);
    reveal(b, lt, 1.8, len - 0.5, 0.6, 40);
  };
});

// ─────────────────────────────────────────────────────────────── 5. select builder
const SEL = [
  `let rows = db\n    .select((users.name, users.email))`,
  `let rows = db\n    .select((users.name, users.email))\n    .from(users)`,
  `let rows = db\n    .select((users.name, users.email))\n    .from(users)\n    .r#where(gt(users.age, 21))`,
  `let rows = db\n    .select((users.name, users.email))\n    .from(users)\n    .r#where(gt(users.age, 21))\n    .order_by(desc(users.age))`,
  `let rows = db\n    .select((users.name, users.email))\n    .from(users)\n    .r#where(gt(users.age, 21))\n    .order_by(desc(users.age))\n    .limit(10)\n    .all()?;`,
  `let rows: «Vec<(String, Option<String>)>» = db\n    .select((users.name, «users.email»))\n    .from(users)\n    .r#where(gt(users.age, 21))\n    .order_by(desc(users.age))\n    .limit(10)\n    .all()?;`,
];
const SQL = [
  `«SELECT "users"."name", "users"."email"»`,
  `SELECT "users"."name", "users"."email"\n«FROM "users"»`,
  `SELECT "users"."name", "users"."email"\nFROM "users"\n«WHERE "users"."age" > ?»`,
  `SELECT "users"."name", "users"."email"\nFROM "users"\nWHERE "users"."age" > ?\n«ORDER BY "users"."age" DESC»`,
  `SELECT "users"."name", "users"."email"\nFROM "users"\nWHERE "users"."age" > ?\nORDER BY "users"."age" DESC\n«LIMIT 10»`,
  `SELECT "users"."name", "users"."email"\nFROM "users"\nWHERE "users"."age" > ?\nORDER BY "users"."age" DESC\nLIMIT 10`,
];
tl.scene(15, (el, len) => {
  const head = header(el, "02 · db.select()", "Reads like SQL. <span class=accent>Because it is.</span>");
  const times = [0.8, 2.6, 4.2, 5.8, 7.4, 10.0];
  const rust = codeWindow(el, 110, 280, { title: "src/main.rs", width: 900, size: 25, height: 380, states: SEL.map((code, i) => ({ at: times[i], code })) });
  const sql = codeWindow(el, 1050, 280, { title: "generated SQL", width: 760, size: 23, lang: "sql", height: 380, states: SQL.map((code, i) => ({ at: times[i], code })) });
  const params = at(el, 1050, 720, h("div", { class: "pill mono", style: { fontSize: "22px" } }, h("span", { style: { color: "var(--muted)" } }, "params"), h("span", { class: "param" }, "[21]")));
  const cap = captions(el, 110, 820, [
    [1.0, 9.6, "One method, one clause. What you write is what runs, with values always bound as parameters."],
    [10.2, len, "The result type is checked against the selection. Nullable <code>email</code> must be <code>Option&lt;String&gt;</code>."],
  ]);
  return (lt) => {
    head(lt, len);
    reveal(rust.win, lt, 0.3, len - 0.5);
    reveal(sql.win, lt, 0.6, len - 0.5);
    rust.update(lt);
    sql.update(lt);
    reveal(params, lt, 4.6, len - 0.5);
    cap(lt);
  };
}, { chapter: "db.select()" });

// ─────────────────────────────────────────────────────────────── 6. aggregates
tl.scene(10, (el, len) => {
  const head = header(el, "02 · db.select()", "Joins and aggregates, <span class=accent>no strings attached.</span>");
  const rust = codeWindow(el, 110, 280, {
    title: "src/report.rs", width: 900, size: 24, height: 330, typeSpeed: 70,
    states: [{ at: 0.6, type: true, code: `
let per_user: Vec<(String, i64)> = db
    .select((users.name, count(posts.id)))
    .from(users)
    .left_join(posts)
    .group_by(users.name)
    .having(gt(count(posts.id), 0))
    .all()?;` }, { at: 6.0, code: `
let per_user: Vec<(String, i64)> = db
    .select((users.name, count(posts.id)))
    .from(users)
    .«left_join(posts)»
    .group_by(users.name)
    .having(gt(count(posts.id), 0))
    .all()?;` }],
  });
  const sql = codeWindow(el, 1050, 280, {
    title: "generated SQL", width: 760, size: 20, lang: "sql", height: 330,
    states: [{ at: 0, code: `
SELECT "users"."name", COUNT("posts"."id")
FROM "users"
LEFT JOIN "posts"
  ON "posts"."author_id" = "users"."id"
GROUP BY "users"."name"
HAVING COUNT("posts"."id") > ?` }, { at: 6.0, code: `
SELECT "users"."name", COUNT("posts"."id")
FROM "users"
LEFT JOIN "posts"
  «ON "posts"."author_id" = "users"."id"»
GROUP BY "users"."name"
HAVING COUNT("posts"."id") > ?` }],
  });
  const cap = captions(el, 110, 820, [
    [1.0, 5.8, "<code>count</code>, <code>sum</code>, <code>avg</code>, <code>group_by</code>, <code>having</code>, subqueries, unions: all typed expressions."],
    [6.2, len, "No <code>ON</code> clause written. The foreign key from your schema fills it in."],
  ]);
  return (lt) => {
    head(lt, len);
    reveal(rust.win, lt, 0.3, len - 0.5);
    reveal(sql.win, lt, 2.5, len - 0.5);
    rust.update(lt);
    sql.update(lt);
    cap(lt);
  };
}, { chapter: "db.select()" });

// ─────────────────────────────────────────────────────────────── 7. dynamic filters
tl.scene(10, (el, len) => {
  const head = header(el, "02 · db.select()", "Optional filters <span class=accent>just compose.</span>");
  const flips = [0.6, 3.6, 6.2];
  const rust = codeWindow(el, 110, 280, {
    title: "src/search.rs", width: 900, size: 24, height: 380,
    states: flips.map((t, i) => ({ at: t, code: `
let search = ${i % 2 === 0 ? '«Some("Ada")»' : "«None»"};
let by_name = search.map(|n| eq(users.name, n));

let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where((gt(users.age, 18), by_name))
    .all()?;` })),
  });
  const sql = codeWindow(el, 1050, 280, {
    title: "generated SQL", width: 760, size: 21, lang: "sql", height: 380,
    states: flips.map((t, i) => ({ at: t, code: `
SELECT "users"."id", "users"."name",
       "users"."email", "users"."age"
FROM "users"
WHERE ("users"."age" > ?${i % 2 === 0 ? `
  «AND "users"."name" = ?»)` : ")"}` })),
  });
  const cap = captions(el, 110, 820, [
    [0.8, len, "A tuple of conditions is an <code>AND</code>. A <code>None</code> drops out. No string building, no <code>if</code> ladders."],
  ]);
  return (lt) => {
    head(lt, len);
    reveal(rust.win, lt, 0.3, len - 0.5);
    reveal(sql.win, lt, 0.5, len - 0.5);
    rust.update(lt);
    sql.update(lt);
    cap(lt);
  };
}, { chapter: "db.select()" });

// ─────────────────────────────────────────────────────────────── 8. query api
tl.scene(15, (el, len) => {
  const head = header(el, "03 · db.query()", "Ask for the shape <span class=accent>you actually want.</span>");
  const rust = codeWindow(el, 110, 280, {
    title: "src/feed.rs", width: 920, size: 24, height: 330, typeSpeed: 60,
    states: [
      { at: 0.6, type: true, code: `
let feed = db
    .query(users)
    .with(users.posts().with(posts.comments()))
    .find_many()?;

for user in &feed {
    println!("{} wrote {}", user.name, user.posts.len());
}` },
      { at: 8.6, code: `
let ada = db
    .query(users)
    .with(users.posts())
    .r#where(eq(users.name, "Ada"))
    .find_first()?; // Option<…>

let post = db.query(posts).with(posts.author()).find_first()?;` },
    ],
  });
  // result tree
  const tx = 1100, ty = 300;
  const node = (x, y, html, cls = "chip") => at(el, tx + x, ty + y, h("div", { class: cls, html }));
  const tree = [
    node(0, 0, '<span class="ty">user</span> <span class="str">"Ada"</span>'),
    node(70, 74, '<span class="p">posts[0]</span> <span class="str">"Hello, drizzle"</span>'),
    node(140, 148, '<span class="p">comments[0]</span> <span class="str">"nice"</span>'),
    node(70, 222, '<span class="p">posts[1]</span> <span class="str">"Zero-cost SQL"</span>'),
    node(140, 296, '<span class="p">comments</span> <span class="p">[]</span>'),
    node(0, 380, '<span class="ty">user</span> <span class="str">"Linus"</span>'),
    node(70, 454, '<span class="p">posts</span> <span class="p">[]</span>'),
  ];
  tree.forEach((n) => (n.style.fontSize = "22px"));
  const lines = at(el, tx, ty, h("div", { html: `<svg width="200" height="520" style="overflow:visible">
    <path d="M28 52 V 110 H 70 M28 110 V 258 H 70 M98 126 V 184 H 140 M98 274 V 332 H 140 M28 432 V 490 H 70" stroke="#2c3644" stroke-width="2" fill="none"/></svg>` }));
  const cap = captions(el, 110, 820, [
    [1.0, 8.2, "Nested relations in one call. <code>user.posts[0].comments</code> is real, typed data."],
    [8.8, len, "Filter, order, paginate, pick columns. Relations work in both directions, generated from <code>references</code>."],
  ]);
  return (lt) => {
    head(lt, len);
    reveal(rust.win, lt, 0.3, len - 0.5);
    rust.update(lt);
    reveal(lines, lt, 3.4, len - 0.5, 0.6, 0);
    stagger(tree, lt, 3.2, 0.22, len - 0.5);
    cap(lt);
  };
}, { chapter: "db.query()" });

// ─────────────────────────────────────────────────────────────── 9. which one?
tl.scene(17, (el, len) => {
  const q1 = at(el, 110, 100, h("div", { class: "kicker" }, "04 · Which one?"));
  const qa = at(el, 110, 142, h("div", { class: "headline", style: { fontSize: "62px" }, html: "“Show me users <span class=accent>with their posts.</span>”" }));
  const qb = at(el, 110, 142, h("div", { class: "headline", style: { fontSize: "62px" }, html: "“How many posts <span class=accent>per user?</span>”" }));
  const rust = codeWindow(el, 110, 290, {
    title: "src/main.rs", width: 900, size: 22, height: 250,
    states: [
      { at: 0, code: `
let rows: Vec<(SelectUsers, Option<SelectPosts>)> = db
    .select(())
    .from(users)
    .left_join(posts)
    .all()?;` },
      { at: 4.6, code: `
let users = db
    .query(users)
    .with(users.posts())
    .find_many()?;` },
      { at: 8.6, code: `
let counts: Vec<(String, i64)> = db
    .select((users.name, count(posts.id)))
    .from(users)
    .left_join(posts)
    .group_by(users.name)
    .all()?;` },
    ],
  });
  // flat rows → tree → counts
  const box = (id, html) => ({ id, html, cls: "chip" });
  const bx = new Boxes(el, 1090, 300, [
    box("u1", '<span class="str">"Ada"</span>'),
    box("u2", '<span class="str">"Ada"</span>'),
    box("u3", '<span class="str">"Linus"</span>'),
    box("p1", '<span class="str">"Hello, drizzle"</span>'),
    box("p2", '<span class="str">"Zero-cost SQL"</span>'),
    box("p3", '<span class="null">None</span>'),
    box("e3", '<span class="p">posts</span> <span class="p">[]</span>'),
    box("c1", '<span class="num">2</span>'),
    box("c3", '<span class="num">0</span>'),
  ]);
  const flat = { u1: [0, 0], p1: [230, 0], u2: [0, 76], p2: [230, 76], u3: [0, 152], p3: [230, 152] };
  const nested = { u1: [0, 0], u2: [0, 0, 0], p1: [80, 76], p2: [80, 152], u3: [0, 250], p3: [80, 326, 0], e3: [80, 326] };
  const counts = { u1: [0, 0], c1: [230, 0], u3: [0, 76], c3: [230, 76] };
  const KF = [{ at: 0, layout: flat }, { at: 5.0, layout: nested }, { at: 9.0, layout: counts }];
  const verdict = (y, html) => at(el, 110, y, h("div", { class: "caption", html }), { width: "900px" });
  const v1 = verdict(600, "<code>select()</code> gives you flat rows: Ada twice, Linus with a <code>None</code>. Regroup them yourself.");
  const v2 = verdict(600, "<code>query()</code> gives you the tree: each user once, posts nested inside, Linus with an empty list.");
  const v3 = verdict(600, "Counting is the database's job. <code>select()</code> + <code>group_by</code>: two numbers come back, not every post.");
  const rule = at(el, 110, 780, h("div", { style: { display: "flex", gap: "24px" } },
    h("div", { class: "card", style: { width: "830px" }, html: `<div class="mono accent" style="font-size:30px;font-weight:700">db.select()</div><div class="sub" style="margin-top:10px;font-size:24px">Think in <b style="color:var(--text)">rows</b>: reports, aggregates, joins across anything, bulk updates.</div>` }),
    h("div", { class: "card", style: { width: "830px" }, html: `<div class="mono accent" style="font-size:30px;font-weight:700">db.query()</div><div class="sub" style="margin-top:10px;font-size:24px">Think in <b style="color:var(--text)">objects</b>: API responses, pages, feeds, anything nested.</div>` })));
  return (lt) => {
    reveal(q1, lt, 0.1, len - 0.5);
    reveal(qa, lt, 0.2, 8.2);
    reveal(qb, lt, 8.6, len - 0.5);
    reveal(rust.win, lt, 0.3, len - 0.5);
    rust.update(lt);
    bx.update(lt, KF);
    bx.items.forEach((it, i) => { if (lt < 1.2 + i * 0.08) it.el.style.opacity = 0; });
    reveal(v1, lt, 1.4, 4.4);
    reveal(v2, lt, 5.6, 8.2);
    reveal(v3, lt, 9.6, 12.8);
    stagger([...rule.children], lt, 13.0, 0.25, len - 0.5);
  };
}, { chapter: "Which one?" });

// ─────────────────────────────────────────────────────────────── 10. type safety
// rustc output: plain text; **bold** marks the error parts, "= note" lines are dimmed.
const errHTML = (txt) =>
  txt
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/\*\*(.+?)\*\*/g, "<b>$1</b>")
    .replace(/^(\s*= note[\s\S]*)$/m, '<span class="note">$1</span>');
const BEATS = [
  {
    code: `let rows: Vec<SelectUsers> = db.select(()).from(users)\n    .r#where(eq(users.age, ‹"thirty"›))\n    .all()?;`,
    err: `**error[E0277]**: SQL type \`drizzle::sqlite::types::Integer\` is not compatible with \`drizzle::sqlite::types::Text\`
   |
   |     .r#where(eq(users.age, "thirty"))
   |              --            **^^^^^^^^ these SQL types cannot be compared or coerced**
   |
   = note: compatible types include: integers with integers/floats,
           text with text/varchar, and any type with itself`,
    say: "Compare a number to a string? Caught at compile time.",
  },
  {
    code: `let rows: Vec<(String, ‹String›)> = db\n    .select((users.name, users.email))\n    .from(users)\n    .all()?;`,
    err: `**error[E0271]**: type mismatch resolving \`<(String, ...) as RowColumnList<...>>::Columns == Cons<String, ...>\`
   |
   |     .all()?;
   |      **^^^ expected \`Cons<String, Cons<Option<String>, Nil>>\`,**
   |          **found \`Cons<String, Cons<String, Nil>>\`**`,
    say: "Forget that <code>email</code> is nullable? It won't compile.",
  },
  {
    code: `db.update(users)\n    .set(‹UpdateUsers::default()›)\n    .r#where(eq(users.id, 1))\n    .execute()?;`,
    err: `**error[E0308]**: mismatched types
   |
   |     .set(UpdateUsers::default())
   |      --- **^^^^^^^^^^^^^^^^^^^^^^ expected \`UpdateUsers<'_, NonEmpty>\`,**
   |                                 **found \`UpdateUsers<'_>\`**`,
    say: "An <code>UPDATE</code> that sets nothing? Not representable.",
  },
  {
    code: `let name = users.name.placeholder("name");\nlet find = db.select(()).from(users).r#where(eq(users.name, name)).prepare();\nfind.all(db.conn(), [name.bind(‹42›)])?;`,
    err: `**error[E0271]**: type mismatch resolving \`<i32 as ValueTypeForDialect<SQLiteDialect>>::SQLType == Text\`
   |
   | find.all(db.conn(), [name.bind(42)])?;
   |                           ---- **^^ expected \`Text\`, found \`Integer\`**`,
    say: "Prepared statements too: placeholders carry their column's type.",
  },
];
tl.scene(21, (el, len) => {
  const head = header(el, "05 · Type safety", "Mistakes <span class=accent>don't reach production.</span>");
  const step = 4.6, t0 = 0.6;
  const beats = BEATS.map((b, i) => {
    const a = t0 + i * step;
    const w = codeWindow(el, 110, 270, { title: "cargo check", width: 1700, size: 26, height: 215, states: [{ at: 0, code: b.code.replace(/[‹›]/g, "") }, { at: 1.1, code: b.code }] });
    const err = at(el, 110, 560, h("div", { class: "err", html: errHTML(b.err) }), { position: "absolute" });
    const say = at(el, 110, 860, h("div", { class: "caption", html: b.say }));
    return { a, w, err, say };
  });
  const list = at(el, 110, 300, h("div", { style: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: "22px 40px", width: "1700px" } },
    ["Column types in comparisons", "Nullability, as Option<T>", "Result rows match the selection", "Required fields on insert", "Non-empty updates", "Typed placeholders"].map((t) =>
      h("div", { class: "card", style: { display: "flex", gap: "20px", alignItems: "center", fontSize: "32px", fontWeight: 600 } }, h("span", { class: "tag ok" }, "compile time"), t))));
  const fin = at(el, 110, 860, h("div", { class: "caption", html: "If it compiles, the SQL is well-typed against your schema." }));
  return (lt) => {
    head(lt, len);
    for (const b of beats) {
      const local = lt - b.a;
      const v = reveal(b.w.win, lt, b.a, b.a + step - 0.45, 0.4, 16);
      b.w.win.style.display = v > 0 ? "" : "none";
      b.w.update(local);
      reveal(b.err, lt, b.a + 1.3, b.a + step - 0.45, 0.4, 16);
      reveal(b.say, lt, b.a + 0.4, b.a + step - 0.45, 0.4, 12);
    }
    const ls = t0 + BEATS.length * step;
    stagger([...list.children], lt, ls, 0.15, len - 0.5);
    reveal(fin, lt, ls + 1.0, len - 0.5);
  };
}, { chapter: "Type safety" });

// ─────────────────────────────────────────────────────────────── 11. ship it
tl.scene(11, (el, len) => {
  const head = header(el, "06 · Ship it", "Migrations from your structs. <span class=accent>Any driver.</span>");
  const term = codeWindow(el, 110, 280, {
    title: "terminal", width: 820, size: 24, lang: "sh", kind: "term", height: 150, typeSpeed: 30,
    states: [{ at: 0.6, type: true, code: `
$ drizzle generate
$ drizzle migrate` }],
  });
  const tree = at(el, 110, 510, h("div", { class: "card mono", style: { width: "820px", fontSize: "22px", lineHeight: "1.7" }, html:
    `<span class="p">drizzle/</span><br>└─ <span class="p">20240101000000_init/</span><br>&nbsp;&nbsp;&nbsp;└─ <span class="accent">migration.sql</span><br><span class="com">// or at startup: db.migrate(&amp;migrations, Tracking::SQLITE)?</span>` }));
  const groups = [
    ["SQLite", ["rusqlite", "libsql", "turso", "Cloudflare D1", "Durable Objects"]],
    ["PostgreSQL", ["postgres", "tokio-postgres", "Hyperdrive", "AWS Data API"]],
    ["MySQL", ["mysql", "mysql_async"]],
  ];
  const cols = groups.map(([name, ds], i) => at(el, 1010, 280 + i * 200, h("div", {},
    h("div", { class: "kicker", style: { color: "var(--muted)", marginBottom: "16px" } }, name),
    h("div", { style: { display: "flex", flexWrap: "wrap", gap: "12px", width: "800px" } }, ds.map((d) => h("span", { class: "pill mono", style: { fontSize: "22px" } }, d))))));
  const cap = captions(el, 110, 900, [[1.0, len, "Generate SQL migrations, apply them from the CLI or at startup. Same query API, sync or async."]]);
  return (lt) => {
    head(lt, len);
    reveal(term.win, lt, 0.3, len - 0.5);
    term.update(lt);
    reveal(tree, lt, 2.2, len - 0.5);
    cols.forEach((c, i) => { reveal(c.firstChild, lt, 3.2 + i * 0.6, len - 0.5); stagger([...c.lastChild.children], lt, 3.4 + i * 0.6, 0.1, len - 0.5); });
    cap(lt);
  };
}, { chapter: "Ship it" });

// ─────────────────────────────────────────────────────────────── 12. outro
tl.scene(7, (el, len) => {
  const logo = at(el, 0, 300, h("div", { class: "logo", style: { fontSize: "150px", width: "1920px", justifyContent: "center" } }, h("span", {}, "drizzle"), h("span", { class: "rs" }, "-rs")));
  const cmd = at(el, 0, 530, h("div", { style: { width: "1920px", display: "flex", justifyContent: "center" } },
    h("div", { class: "window term", style: { padding: "26px 40px", font: '500 38px "JetBrains Mono"' }, html: '<span class="prompt">$</span> cargo add drizzle <span class="attr">--features</span> rusqlite' })));
  const url = at(el, 0, 720, h("div", { class: "sub mono", style: { width: "1920px", textAlign: "center", fontSize: "30px" } }, "github.com/themixednuts/drizzle-rs"));
  const tag = at(el, 0, 790, h("div", { class: "sub", style: { width: "1920px", textAlign: "center" }, html: "SQL you know. Types you trust." }));
  return (lt) => {
    reveal(logo, lt, 0.1, len - 0.7, 0.8, 30);
    reveal(cmd, lt, 0.8, len - 0.7);
    reveal(url, lt, 1.5, len - 0.7);
    reveal(tag, lt, 2.0, len - 0.7);
  };
}, { fadeOut: 0.8 });

stage(tl, root, CH);
boot(tl);
