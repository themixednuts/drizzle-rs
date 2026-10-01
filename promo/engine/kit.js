// Scene-building helpers shared by both videos.
import { h, prog, ease, lerp, reveal, frame, Code } from "./engine.js";

/** Absolutely positioned element. */
export function at(parent, x, y, el, extra = {}) {
  Object.assign(el.style, { position: "absolute", left: x + "px", top: y + "px", ...extra });
  parent.append(el);
  return el;
}

/** Kicker + headline at the top of a scene; returns an updater. */
export function header(parent, kicker, headline, { y = 90, size } = {}) {
  const k = at(parent, 110, y, h("div", { class: "kicker" }, kicker));
  const t = at(parent, 110, y + 42, h("div", { class: "headline", html: headline }));
  if (size) t.style.fontSize = size + "px";
  return (lt, len) => {
    reveal(k, lt, 0.1, len - 0.6);
    reveal(t, lt, 0.2, len - 0.6);
  };
}

/** Caption line that swaps text at given times, crossfading. */
export function captions(parent, x, y, items, { cls = "caption", width = 1700 } = {}) {
  const els = items.map(([a, b, html]) => {
    const el = at(parent, x, y, h("div", { class: cls, html }), { width: width + "px" });
    return { a, b, el };
  });
  return (lt) => {
    for (const c of els) reveal(c.el, lt, c.a, c.b, 0.45, 14);
  };
}

/** A code window. Returns { win, code, update }. */
export function codeWindow(parent, x, y, { title, width, states, size = 24, lh, lang = "rust", kind, typeSpeed, dur, height }) {
  const code = new Code({ lang, size, lh, states, typeSpeed, dur });
  const win = frame({ title, width, kind: kind || (lang === "sql" ? "sql" : "editor"), children: code.el });
  at(parent, x, y, win);
  if (height) win.querySelector(".body").style.height = height + "px";
  code.measure();
  return { win, code, update: (lt) => code.update(lt) };
}

/** Shows a set of boxes that move between named layouts. */
export class Boxes {
  constructor(parent, x, y, items) {
    this.items = items.map((it) => {
      const el = h("div", { class: it.cls || "box", html: it.html });
      Object.assign(el.style, { position: "absolute", left: 0, top: 0 });
      parent.append(el);
      return { ...it, el };
    });
    this.x = x;
    this.y = y;
  }
  /** keyframes: [{ at, layout: {id: [x, y, opacity?]} }] */
  update(lt, keyframes, dur = 0.9) {
    let i = 0;
    while (i + 1 < keyframes.length && lt >= keyframes[i + 1].at) i++;
    const cur = keyframes[i].layout;
    const prev = i > 0 ? keyframes[i - 1].layout : cur;
    const p = i > 0 ? prog(lt, keyframes[i].at, keyframes[i].at + dur, ease.inOut) : 1;
    for (const it of this.items) {
      let a = prev[it.id], b = cur[it.id];
      if (!a && !b) { it.el.style.opacity = 0; continue; }
      if (!a) a = [b[0], b[1], 0];
      if (!b) b = [a[0], a[1], 0];
      const oa = a[2] ?? 1, ob = b[2] ?? 1;
      const x = lerp(a[0], b[0], p), y = lerp(a[1], b[1], p);
      it.el.style.transform = `translate(${this.x + x}px, ${this.y + y}px)`;
      it.el.style.opacity = lerp(oa, ob, p);
    }
  }
}

/** A labelled pair of stacked chips/cards that pop in at time `a` with stagger. */
export function stagger(els, lt, a, step = 0.12, b = Infinity) {
  els.forEach((el, i) => reveal(el, lt, a + i * step, b, 0.45, 18));
}

export { h, prog, ease, lerp, reveal };
