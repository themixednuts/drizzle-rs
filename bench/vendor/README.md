# Vendored crates

Third-party sources kept here only because they carry a patch we cannot get any other way. Each
one is the published release with a single, documented change, and each should leave as soon as
that change lands upstream.

## `libsql-sqlite3-parser` 0.13.0

One line in `build.rs`. The script compiles its `lemon` parser generator with the host compiler and
`-o`, which places the executable in `OUT_DIR` correctly — but MSVC's `cl.exe` additionally writes
an intermediate `lemon.obj` beside its *current directory*, and a build script's current directory
is the crate's own source directory. Under the cargo registry that directory is shared by every
build on the machine, so two cargo processes compiling this crate concurrently — a `cargo build`
running alongside rust-analyzer is enough — collide on that one path, and the loser trips the
script's `assert!`. It is intermittent, only ever affects a first build, and never happens on gcc
or clang, neither of which leaves an object file behind.

The patch adds `/Fo<OUT_DIR>/lemon.obj` when the compiler is MSVC-like, so the intermediate lands
somewhere per-build. The gcc and clang command lines are untouched.

This is wired up through `[patch.crates-io]` in the workspace root, which is the only place cargo
honours one. It changes nothing for the rest of the repository: `libsql-sqlite3-parser` is reachable
only with the `libsql` feature, which is off by default and used only by the benchmark runner's
libsql family, so an ordinary build resolves exactly as it did before.

Upstream fix: give the `Command` a `.current_dir(&out_dir)` with an absolute source path, or emit
`/Fo` as here. No newer semver-compatible release carries it as of 0.13.0.
