// Render a promo page to MP4: step `seek(t)` frame by frame, screenshot, pipe to ffmpeg.
//
//   node render.mjs standalone            -> out/drizzle-rs.mp4
//   node render.mjs comparison            -> out/drizzle-rs-vs.mp4
//   node render.mjs standalone --still 42 -> out/standalone-42.png (one frame)
//
// Needs Playwright (with Chromium) and ffmpeg on PATH.
import { createRequire } from "node:module";
import { spawn, execSync } from "node:child_process";
import { createServer } from "node:http";
import { readFile, mkdir } from "node:fs/promises";
import { extname, join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); }
catch { ({ chromium } = require(join(execSync("npm root -g").toString().trim(), "playwright"))); }

const [name = "standalone", ...rest] = process.argv.slice(2);
const flag = (f) => { const i = rest.indexOf(f); return i >= 0 ? rest[i + 1] : undefined; };
const OUT = { standalone: "drizzle-rs.mp4", comparison: "drizzle-rs-vs.mp4" };
const workers = Number(flag("--workers") || 4);
const fps = Number(flag("--fps") || 30);

const MIME = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css", ".woff2": "font/woff2" };
const server = createServer(async (req, res) => {
  try {
    const path = join(here, decodeURIComponent(new URL(req.url, "http://x").pathname));
    res.writeHead(200, { "content-type": MIME[extname(path)] || "application/octet-stream" });
    res.end(await readFile(path));
  } catch { res.writeHead(404); res.end(); }
}).listen(0);
const url = `http://127.0.0.1:${server.address().port}/${name}.html`;

const browser = await chromium.launch();
async function openPage() {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on("pageerror", (e) => console.error("page error:", e.message));
  await page.goto(url);
  await page.waitForFunction(() => window.__ready === true);
  return page;
}

await mkdir(join(here, "out"), { recursive: true });
const still = flag("--still");
if (still !== undefined) {
  const page = await openPage();
  await page.evaluate((t) => window.seek(t), Number(still));
  const file = join(here, "out", `${name}-${still}.png`);
  await page.screenshot({ path: file });
  console.log(file);
  await browser.close(); server.close();
  process.exit(0);
}

const pages = await Promise.all(Array.from({ length: workers }, openPage));
const duration = await pages[0].evaluate(() => window.__duration);
const frames = Math.ceil(duration * fps);
const out = join(here, "out", flag("--out") || OUT[name] || `${name}.mp4`);
console.log(`${name}: ${duration.toFixed(1)}s, ${frames} frames -> ${out}`);

const ff = spawn("ffmpeg", [
  "-y", "-loglevel", "error", "-f", "image2pipe", "-framerate", String(fps), "-c:v", "png", "-i", "-",
  "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out,
], { stdio: ["pipe", "inherit", "inherit"] });

// workers render ahead; frames are written strictly in order
const done = new Map();
let next = 0, written = 0;
const write = () => {
  while (done.has(written)) {
    ff.stdin.write(done.get(written));
    done.delete(written++);
    if (written % (fps * 5) === 0) process.stdout.write(`  ${(written / fps).toFixed(0)}s\r`);
  }
};
await Promise.all(pages.map(async (page) => {
  while (next < frames) {
    const f = next++;
    await page.evaluate((t) => window.seek(t), f / fps);
    done.set(f, await page.screenshot({ type: "png" }));
    write();
    while (done.size > workers * 8) await new Promise((r) => setTimeout(r, 5));
  }
}));
write();
ff.stdin.end();
await new Promise((r) => ff.on("close", r));
await browser.close();
server.close();
console.log(`\ndone: ${out}`);
