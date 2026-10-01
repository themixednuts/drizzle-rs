import { h, prog, clamp, ease } from "./engine.js";

/** Background glow + grid, and a chapter progress bar along the bottom. */
export function stage(tl, root, chapters) {
  const bg = h("div", { id: "bg" });
  const g1 = h("div", { class: "glow", style: { background: "radial-gradient(circle, rgba(197,247,79,0.10), transparent 60%)" } });
  const g2 = h("div", { class: "glow", style: { background: "radial-gradient(circle, rgba(94,234,212,0.07), transparent 60%)" } });
  const grid = h("div", { class: "grid" });
  bg.append(g1, g2, grid);
  root.prepend(bg);

  tl.track((t) => {
    g1.style.transform = `translate(${-300 + Math.sin(t * 0.13) * 160}px, ${-500 + Math.cos(t * 0.11) * 120}px)`;
    g2.style.transform = `translate(${900 + Math.cos(t * 0.09) * 200}px, ${200 + Math.sin(t * 0.12) * 140}px)`;
    grid.style.transform = `translate(${(t * 6) % 80}px, ${(t * 3) % 80}px)`;
  });

  if (!chapters) return;
  const bar = h("div", { id: "chapters" });
  root.append(bar);
  // chapter spans come from scene options; resolved lazily once all scenes exist
  let spans = null;
  const els = chapters.map((name) => {
    const fill = h("div", { class: "fill" });
    const el = h("div", { class: "ch" }, h("div", { class: "lbl" }, name), h("div", { class: "trk" }, fill));
    bar.append(el);
    return { name, el, fill };
  });
  tl.track((t) => {
    if (!spans) {
      spans = {};
      for (const s of tl.scenes) {
        if (!s.chapter) continue;
        const sp = (spans[s.chapter] ||= { a: s.start, b: s.start + s.len });
        sp.a = Math.min(sp.a, s.start);
        sp.b = Math.max(sp.b, s.start + s.len);
      }
    }
    const first = Math.min(...Object.values(spans).map((s) => s.a));
    const last = Math.max(...Object.values(spans).map((s) => s.b));
    const vis = prog(t, first - 0.6, first, ease.out) * (1 - prog(t, last - 0.3, last + 0.3, ease.in));
    bar.style.opacity = vis;
    for (const c of els) {
      const sp = spans[c.name];
      const p = sp ? clamp((t - sp.a) / (sp.b - sp.a)) : 0;
      c.fill.style.width = p * 100 + "%";
      c.el.classList.toggle("on", !!sp && t >= sp.a && t < sp.b);
    }
  });
}
