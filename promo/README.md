# Promo videos

Two 1080p promo videos for drizzle-rs, built as HTML animations and rendered
frame by frame to MP4.

| Video | Source | Output |
|---|---|---|
| drizzle-rs: schema, `db.select()` vs `db.query()`, type safety, migrations | `standalone.js` | `out/drizzle-rs.mp4` (not committed) |
| drizzle-rs vs SeaORM, Diesel and Toasty | `comparison.js` | `out/drizzle-rs-vs.mp4` (not committed) |

## Render

Needs Node 20+, Playwright with Chromium, and ffmpeg.

```bash
cd promo
node render.mjs standalone              # -> out/drizzle-rs.mp4
node render.mjs comparison              # -> out/drizzle-rs-vs.mp4
node render.mjs standalone --still 42   # one frame at t=42s -> out/standalone-42.png
```

`--workers N` sets how many browser pages render in parallel (default 4) and
`--fps` the frame rate (default 30).

To preview while editing, serve this directory (`npx serve promo`) and open
`standalone.html?play`, or `standalone.html?t=42` and step with the arrow keys
(shift for 5s jumps).

## How it works

- `engine/engine.js`: every visual is a pure function of time. `seek(t)` puts
  the page in its state at `t` seconds, so frames are deterministic. It also
  holds the Rust/SQL highlighter and `Code`, a code block that types itself
  in or morphs between versions of a snippet (matching lines glide to their
  new row).
- Inline marks in snippet strings: `«…»` highlights, `‹…›` draws an error
  squiggle, `⟨…⟩` dims.
- `engine/kit.js` has scene layout helpers; `engine/stage.js` draws the
  background and the chapter bar.
- `render.mjs` serves the directory, opens N Playwright pages, steps
  `seek(f / fps)` and pipes PNG frames to ffmpeg in order.

## Keeping the code honest

Every drizzle snippet on screen compiles against this repo:
`verify/` is a standalone crate holding them all, and its output is the SQL
the videos quote. The compiler errors shown are real `cargo check` output
(long type paths in the middle of a message are elided with `...`, as rustc
itself does).

```bash
cd promo/verify && cargo run
```

The SeaORM, Diesel and Toasty snippets were compiled against the crate
versions listed in `comparison.js`, starting from the projects in
`examples/orm-comparison/`.

Fonts: Inter and JetBrains Mono (SIL Open Font License), in `assets/fonts/`.
