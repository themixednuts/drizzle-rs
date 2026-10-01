// Tiny deterministic animation engine for the promo videos.
//
// Every visual is a pure function of time: `seek(t)` puts the whole page into
// the state it has `t` seconds into the video. The renderer (render.mjs) steps
// `t` frame by frame and screenshots, so output never depends on wall-clock
// timing, and the page can also be scrubbed live in a browser (?t=12.5, or the
// arrow keys) while editing.

export const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x));
export const lerp = (a, b, p) => a + (b - a) * p;
export const ease = {
  linear: (x) => x,
  out: (x) => 1 - Math.pow(1 - x, 3),
  in: (x) => x * x * x,
  inOut: (x) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2),
  back: (x) => {
    const c1 = 1.70158, c3 = c1 + 1;
    return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2);
  },
};
/** Progress of `t` through [a, b], eased. */
export const prog = (t, a, b, e = ease.out) => e(clamp((t - a) / (b - a)));

export const h = (tag, attrs = {}, ...kids) => {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") el.className = v;
    else if (k === "style") Object.assign(el.style, v);
    else if (k === "html") el.innerHTML = v;
    else el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) {
    if (kid == null) continue;
    el.append(kid instanceof Node ? kid : document.createTextNode(kid));
  }
  return el;
};

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

// ---------------------------------------------------------------- highlighting

const RUST_KW = new Set(
  "let fn pub struct use mut for in if else match return async await impl mod as where type enum crate self super move const dyn loop while ref"
    .split(" "),
);
const RUST_LIT = new Set(["Some", "None", "Ok", "Err", "true", "false", "Self"]);
const SQL_KW = new Set(
  "SELECT FROM WHERE AND OR NOT ORDER BY GROUP HAVING LIMIT OFFSET LEFT INNER JOIN ON AS DESC ASC INSERT INTO VALUES UPDATE SET DELETE CREATE TABLE PRIMARY KEY AUTOINCREMENT NULL INTEGER TEXT CONSTRAINT FOREIGN REFERENCES COUNT IN EXISTS RETURNING"
    .split(" "),
);

function tokenizeRust(src) {
  const out = [];
  const re =
    /(\/\/[^\n]*)|("(?:\\.|[^"\\])*")|('[a-z_]+\b(?!'))|(#!?\[)|(r#[A-Za-z_]\w*|[A-Za-z_]\w*)(!?)|(\d[\d_]*(?:\.\d+)?(?:i64|i32|u64)?)|(\s+)|(\?)|([^\w\s])/gy;
  let m, attrDepth = 0, prev = "";
  while ((m = re.exec(src))) {
    const [all, com, str, life, attrOpen, id, bang, num, ws, q, p] = m;
    let cls = "";
    if (com) cls = "com";
    else if (str) cls = "str";
    else if (life) cls = "kw";
    else if (attrOpen) { cls = "attr"; attrDepth = 1; }
    else if (id) {
      const after = src.slice(re.lastIndex).match(/^\s*(\(|::<|<)/);
      if (bang) cls = "mac";
      else if (attrDepth) cls = RUST_KW.has(id) ? "kw" : /^[A-Z]/.test(id) ? "ty" : "attrk";
      else if (RUST_KW.has(id)) cls = "kw";
      else if (RUST_LIT.has(id)) cls = "lit";
      else if (/^[A-Z]/.test(id)) cls = "ty";
      else if (after && after[1] === "(") cls = "fn";
      else if (prev === ".") cls = "prop";
      else cls = "id";
    } else if (num) cls = "num";
    else if (ws) cls = "";
    else if (q) cls = "q";
    else if (p) {
      cls = attrDepth ? "attr" : "p";
      if (attrDepth && p === "[") attrDepth++;
      if (attrDepth && p === "]") { attrDepth--; cls = "attr"; }
    }
    out.push({ text: all, cls });
    if (!ws) prev = all;
  }
  return out;
}

function tokenizeSql(src) {
  const out = [];
  const re = /("[^"]*"|`[^`]*`)|(--[^\n]*)|([A-Za-z_]\w*)|(\d+)|(\?)|(\s+)|(.)/gy;
  let m;
  while ((m = re.exec(src))) {
    const [all, quoted, com, word, num, q, ws] = m;
    let cls = "p";
    if (quoted) cls = "sqlid";
    else if (com) cls = "com";
    else if (word) cls = SQL_KW.has(word.toUpperCase()) ? "sqlkw" : "id";
    else if (num) cls = "num";
    else if (q) cls = "param";
    else if (ws) cls = "";
    out.push({ text: all, cls });
  }
  return out;
}

function tokenizeShell(src) {
  return src.split(/(\s+)/).map((text, i) => ({
    text,
    cls: text.startsWith("$") ? "prompt" : text.startsWith("--") ? "attr" : i === 2 ? "fn" : "",
  }));
}

const TOKENIZERS = { rust: tokenizeRust, sql: tokenizeSql, sh: tokenizeShell, plain: (s) => [{ text: s, cls: "" }] };

/**
 * Inline marks inside code strings: «text» highlights a span, ‹text› gets an
 * error squiggle, ⟨text⟩ dims it. Marks are stripped before tokenizing and
 * re-applied by character range, so they never disturb highlighting.
 */
const MARKS = { "«": ["»", "mark"], "‹": ["›", "squig"], "⟨": ["⟩", "dim"] };

function parseMarks(line) {
  let clean = "";
  const ranges = [];
  const stack = [];
  for (const ch of line) {
    if (MARKS[ch]) stack.push({ start: clean.length, cls: MARKS[ch][1], close: MARKS[ch][0] });
    else if (stack.length && ch === stack[stack.length - 1].close) {
      const r = stack.pop();
      ranges.push({ start: r.start, end: clean.length, cls: r.cls });
    } else clean += ch;
  }
  return { clean, ranges };
}

/** Highlighted HTML for one line. `limit` truncates to that many characters (typing). */
export function lineHTML(line, lang = "rust", limit = Infinity) {
  const { clean, ranges } = parseMarks(line);
  const toks = TOKENIZERS[lang](clean);
  let pos = 0, html = "";
  for (const tok of toks) {
    // split the token at mark boundaries
    let s = 0;
    while (s < tok.text.length) {
      const abs = pos + s;
      if (abs >= limit) return html;
      let e = tok.text.length;
      const extra = [];
      for (const r of ranges) {
        if (abs >= r.start && abs < r.end) { extra.push(r.cls); e = Math.min(e, r.end - pos); }
        else if (r.start > abs) e = Math.min(e, r.start - pos);
      }
      e = Math.min(e, limit - pos);
      const piece = tok.text.slice(s, e);
      const cls = [tok.cls, ...extra].filter(Boolean).join(" ");
      html += cls ? `<span class="${cls}">${esc(piece)}</span>` : esc(piece);
      s = e;
    }
    pos += tok.text.length;
  }
  return html;
}

export const plainLen = (line) => parseMarks(line).clean.length;

// ---------------------------------------------------------------- code views

/**
 * A code block that morphs between several versions of a snippet ("magic
 * move"): lines are matched by content, kept lines glide to their new row,
 * new lines fade in, dropped lines fade out.
 *
 * states: [{ at, code, type? }] — at `at` seconds (scene-local) the block
 * transitions to `code` over `dur`. `type: true` types the new lines in.
 */
export class Code {
  constructor({ lang = "rust", size = 26, lh = 1.55, states, dur = 0.7, typeSpeed = 55, gutter = false }) {
    this.lang = lang;
    this.size = size;
    this.lhPx = Math.round(size * lh);
    this.states = states.map((s) => ({ ...s, lines: s.code.replace(/^\n/, "").replace(/\s+$/, "").split("\n") }));
    this.dur = dur;
    this.typeSpeed = typeSpeed;
    this.el = h("div", { class: "code", style: { fontSize: size + "px", lineHeight: this.lhPx + "px" } });
    this.rows = new Map(); // key -> div
    this.keys = this.states.map((s) => this.keyLines(s.lines));
    const maxLines = Math.max(...this.states.map((s) => s.lines.length));
    this.el.style.height = maxLines * this.lhPx + "px";
    this.gutter = gutter;
  }
  keyLines(lines) {
    const seen = {};
    return lines.map((l) => {
      const k = l.trim() === "" ? "∅" : parseMarks(l).clean;
      seen[k] = (seen[k] || 0) + 1;
      return k + "#" + seen[k];
    });
  }
  row(key) {
    if (!this.rows.has(key)) {
      const d = h("div", { class: "row" });
      this.el.append(d);
      this.rows.set(key, d);
    }
    return this.rows.get(key);
  }
  /** Fraction typed for the state at index i. */
  typed(i, lt) {
    const s = this.states[i];
    if (!s.type) return Infinity;
    return (lt - s.at) * this.typeSpeed;
  }
  update(lt) {
    let i = 0;
    while (i + 1 < this.states.length && lt >= this.states[i + 1].at) i++;
    const cur = this.states[i];
    const prev = i > 0 ? this.states[i - 1] : null;
    const p = prev && !cur.type ? prog(lt, cur.at, cur.at + this.dur, ease.inOut) : 1;
    const curKeys = this.keys[i];
    const prevKeys = prev ? this.keys[i - 1] : [];
    const live = new Set();
    // typing budget for new lines in this state
    let budget = cur.type ? this.typed(i, lt) : Infinity;
    const prevSet = new Set(prevKeys);
    curKeys.forEach((k, li) => {
      const d = this.row(k);
      live.add(k);
      const y1 = li * this.lhPx;
      const pi = prevKeys.indexOf(k);
      const y0 = pi >= 0 ? pi * this.lhPx : y1;
      const isNew = pi < 0;
      let limit = Infinity;
      if (cur.type && !prevSet.has(k)) {
        const len = plainLen(cur.lines[li]);
        limit = Math.max(0, budget);
        budget -= len + 4; // short pause at end of line
      }
      const html = lineHTML(cur.lines[li], this.lang, limit);
      if (d._html !== html) { d.innerHTML = html || "&#8203;"; d._html = html; }
      const typingHere = !!cur.type && isNew && limit > 0 && limit < plainLen(cur.lines[li]);
      d.classList.toggle("caret", typingHere);
      d.style.transform = `translateY(${lerp(y0, y1, p)}px)`;
      d.style.opacity = isNew && !cur.type ? p : cur.type && isNew && limit <= 0 ? 0 : 1;
      if (isNew && !cur.type) d.style.transform += ` translateX(${(1 - p) * 18}px)`;
    });
    for (const [k, d] of this.rows) {
      if (live.has(k)) continue;
      const pi = prevKeys.indexOf(k);
      if (pi >= 0 && p < 1) {
        d.style.opacity = 1 - p;
        d.style.transform = `translateY(${pi * this.lhPx}px) translateX(${-p * 18}px)`;
      } else d.style.opacity = 0;
    }
  }
  /** Pixel position of (line, col) in the current layout, relative to the block. */
  at(line, col) {
    return { x: col * this.charW, y: line * this.lhPx };
  }
  measure() {
    const probe = h("span", { style: { visibility: "hidden", position: "absolute" } }, "0".repeat(100));
    this.el.append(probe);
    this.charW = probe.getBoundingClientRect().width / 100;
    probe.remove();
  }
}

/** A macOS-style window around content. */
export function frame({ title = "", kind = "editor", width, children, style = {} }) {
  const bar = h(
    "div",
    { class: "bar" },
    h("i", { class: "dot r" }), h("i", { class: "dot y" }), h("i", { class: "dot g" }),
    h("span", { class: "title" }, title),
  );
  const body = h("div", { class: "body" }, children);
  return h("div", { class: `window ${kind}`, style: { width: width ? width + "px" : undefined, ...style } }, bar, body);
}

// ---------------------------------------------------------------- timeline

export class Timeline {
  constructor(root, { fps = 30 } = {}) {
    this.root = root;
    this.fps = fps;
    this.scenes = [];
    this.tracks = []; // global updaters (background, chapter bar)
    this.duration = 0;
  }
  /** Add a scene lasting `len` seconds. `build(el)` returns `update(lt, len)`. */
  scene(len, build, { fadeIn = 0.45, fadeOut = 0.45, chapter } = {}) {
    const start = this.duration;
    const el = h("section", { class: "scene" });
    this.root.append(el);
    const update = build(el, len) || (() => {});
    this.scenes.push({ start, len, el, update, fadeIn, fadeOut, chapter });
    this.duration += len;
    return start;
  }
  track(fn) { this.tracks.push(fn); }
  seek(t) {
    for (const s of this.scenes) {
      const lt = t - s.start;
      const on = lt >= -0.0001 && lt < s.len;
      s.el.style.display = on ? "" : "none";
      if (!on) continue;
      const a = Math.min(prog(lt, 0, s.fadeIn || 1e-6), 1 - prog(lt, s.len - (s.fadeOut || 1e-6), s.len, ease.in));
      s.el.style.opacity = a;
      s.update(lt, s.len);
    }
    for (const f of this.tracks) f(t);
  }
  current(t) { return this.scenes.find((s) => t >= s.start && t < s.start + s.len); }
}

/** Fade + rise an element in over [a, a+d], and optionally out at [b, b+d]. */
export function reveal(el, lt, a, b = Infinity, d = 0.5, rise = 24) {
  const pin = prog(lt, a, a + d);
  const pout = prog(lt, b, b + d, ease.in);
  const v = pin * (1 - pout);
  el.style.opacity = v;
  el.style.transform = `translateY(${(1 - pin) * rise - pout * rise * 0.5}px)`;
  return v;
}

/** Wire up the page: fonts, `window.seek`, scrubbing with ?t= and arrow keys. */
export async function boot(tl, onReady = () => {}) {
  await document.fonts.ready;
  onReady();
  window.__duration = tl.duration;
  window.__fps = tl.fps;
  window.seek = (t) => tl.seek(t);
  const q = new URLSearchParams(location.search);
  let t = parseFloat(q.get("t") || "0");
  tl.seek(t);
  if (q.has("play")) {
    const t0 = performance.now() - t * 1000;
    const loop = () => { t = (performance.now() - t0) / 1000; tl.seek(t % tl.duration); requestAnimationFrame(loop); };
    requestAnimationFrame(loop);
  }
  addEventListener("keydown", (e) => {
    if (e.key === "ArrowRight") t += e.shiftKey ? 5 : 0.5;
    if (e.key === "ArrowLeft") t -= e.shiftKey ? 5 : 0.5;
    t = clamp(t, 0, tl.duration);
    tl.seek(t);
    document.title = t.toFixed(2);
  });
  window.__ready = true;
}
