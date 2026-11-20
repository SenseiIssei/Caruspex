"""Measures the interface in a headless Chromium: start, renders, memory, WebGL.

Not part of the build or the CI. It needs Python Playwright with its Chromium
(`pip install playwright` and `playwright install chromium`), installed by
whoever measures; the project itself does not depend on it. The interface
runs with the demo backend in the browser preview:

    npm --prefix app run build && npx --prefix app vite preview --port 5392   (release build)
    npm --prefix app run dev                                                   (port 5287, for "xray")

    py -I tools/ui_measure.py startup http://localhost:5392 [4]   first paint, ready, long tasks; 4 = CPU four times slower
    py -I tools/ui_measure.py live http://localhost:5392 [page]   renders and CPU per second while values stream
                                                                  (page: live-run for the live page, start-live for the start page)
    py -I tools/ui_measure.py long http://localhost:5392 10       heap, DOM nodes and listeners over a 10 minute live session
    py -I tools/ui_measure.py xray http://localhost:5287          WebGL objects after 20 visits of the X-ray page and 20 rebuilds
    py -I tools/ui_measure.py xraylive http://localhost:5287      WebGL objects made per second while packets fly

Renders are counted through a stand-in for the React DevTools hook, so the
app needs no change and the release build is measured as shipped; with the
development server the component names are readable. docs/performance.md
has the numbers and what they mean.
"""
import json
import statistics
import sys
import time

from playwright.sync_api import sync_playwright

ARGS = [
    "--enable-precise-memory-info",
    "--js-flags=--expose-gc",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--ignore-gpu-blocklist",
]

HOOK = r"""
(() => {
  const stats = { commits: 0, renders: 0, byName: {}, byPage: {} };
  const pageOf = (f) => {
    for (let p = f; p; p = p.return) {
      const props = p.memoizedProps;
      if (p.tag === 5 && props && props["data-page"]) return props["data-page"];
    }
    return "(shell)";
  };
  window.__perf = stats;
  const nameOf = (f) => {
    const t = f.type;
    if (!t) return "?";
    return t.displayName || t.name || (t.render && (t.render.displayName || t.render.name)) || (t.type && (t.type.displayName || t.type.name)) || "anon";
  };
  // A component rendered in this commit when it is new or did work; subtrees
  // whose children were not reconciled (child pointer unchanged) are skipped.
  const walk = (root) => {
    const stack = [root];
    while (stack.length) {
      const f = stack.pop();
      const tag = f.tag;
      if (tag === 0 || tag === 1 || tag === 11 || tag === 14 || tag === 15) {
        if (!f.alternate || (f.flags & 1)) {
          stats.renders++;
          const n = nameOf(f);
          stats.byName[n] = (stats.byName[n] || 0) + 1;
          const pg = pageOf(f);
          stats.byPage[pg] = (stats.byPage[pg] || 0) + 1;
        }
      }
      if (!f.alternate || f.child !== f.alternate.child) {
        for (let c = f.child; c; c = c.sibling) stack.push(c);
      }
    }
  };
  window.__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
    renderers: new Map(), supportsFiber: true, isDisabled: false,
    inject(r) { const id = this.renderers.size + 1; this.renderers.set(id, r); return id; },
    onCommitFiberRoot(id, root) { stats.commits++; try { walk(root.current); } catch (e) {} },
    onCommitFiberUnmount() {}, onPostCommitFiberRoot() {}, checkDCE() {}, on() {}, off() {}, emit() {}, sub() { return () => {}; },
  };
  window.__lt = [];
  try {
    new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__lt.push([e.startTime, e.duration]); }).observe({ type: "longtask", buffered: true });
  } catch (e) {}
  window.__ready = null;
  const check = () => { if (!window.__ready && document.querySelector(".sidebar .nav-button")) window.__ready = performance.now(); };
  new MutationObserver(check).observe(document, { childList: true, subtree: true });
})();
"""

GL_HOOK = r"""
(() => {
  const live = {};
  const made = { uploads: 0 };
  window.__gl = live;
  window.__glMade = made;
  const pairs = [["createBuffer", "deleteBuffer", "buffers"], ["createTexture", "deleteTexture", "textures"], ["createProgram", "deleteProgram", "programs"], ["createFramebuffer", "deleteFramebuffer", "framebuffers"], ["createVertexArray", "deleteVertexArray", "vaos"]];
  for (const proto of [WebGLRenderingContext.prototype, WebGL2RenderingContext.prototype]) {
    for (const [mk, rm, key] of pairs) {
      const c = proto[mk], d = proto[rm];
      if (!c || !d) continue;
      live[key] = 0;
      made[key] = 0;
      proto[mk] = function (...a) { const o = c.apply(this, a); if (o) { live[key]++; made[key]++; } return o; };
      proto[rm] = function (o) { if (o) live[key]--; return d.call(this, o); };
    }
    for (const up of ["texImage2D", "texSubImage2D", "bufferData"]) {
      const f = proto[up];
      if (!f) continue;
      proto[up] = function (...a) { made.uploads++; return f.apply(this, a); };
    }
  }
})();
"""


def metrics(cdp):
    return {m["name"]: m["value"] for m in cdp.send("Performance.getMetrics")["metrics"]}


def gc(page, cdp):
    cdp.send("HeapProfiler.collectGarbage")
    page.wait_for_timeout(300)
    cdp.send("HeapProfiler.collectGarbage")


def new_page(browser, extra=None):
    ctx = browser.new_context(viewport={"width": 1440, "height": 920})
    page = ctx.new_page()
    page.add_init_script(HOOK)
    if extra:
        page.add_init_script(extra)
    cdp = ctx.new_cdp_session(page)
    cdp.send("Performance.enable")
    return ctx, page, cdp


def startup(browser, base, slow=1):
    rows = []
    for _ in range(6):
        ctx, page, cdp = new_page(browser)
        # A weak laptop: the CPU four times slower.
        if slow > 1:
            cdp.send("Emulation.setCPUThrottlingRate", {"rate": slow})
        page.goto(f"{base}/?lang=en", wait_until="load")
        page.wait_for_function("window.__ready !== null", timeout=30000)
        page.wait_for_timeout(3000)
        r = page.evaluate(
            """() => {
              const paint = Object.fromEntries(performance.getEntriesByType('paint').map(e => [e.name, e.startTime]));
              const nav = performance.getEntriesByType('navigation')[0];
              const fcp = paint['first-contentful-paint'] || 0;
              const lt = window.__lt;
              const tbt = lt.filter(([s]) => s >= fcp).reduce((a, [, d]) => a + Math.max(0, d - 50), 0);
              const lastLong = lt.reduce((a, [s, d]) => Math.max(a, s + d), 0);
              return { fp: paint['first-paint'] || 0, fcp, ready: window.__ready, dcl: nav.domContentLoadedEventEnd, tti: Math.max(window.__ready, lastLong), tbt, longTasks: lt.length };
            }"""
        )
        gc(page, cdp)
        m = metrics(cdp)
        r["heapMB"] = m["JSHeapUsedSize"] / 1e6
        r["nodes"] = m["Nodes"]
        r["listeners"] = m["JSEventListeners"]
        r["commits"] = page.evaluate("window.__perf.commits")
        r["renders"] = page.evaluate("window.__perf.renders")
        rows.append(r)
        ctx.close()
    rows = rows[1:]  # the first run warms the server and the disk cache
    out = {k: round(statistics.median(r[k] for r in rows), 1) for k in rows[0]}
    print(json.dumps(out))


def live(browser, base, which="live-run", seconds=10):
    ctx, page, cdp = new_page(browser)
    page.goto(f"{base}/?demo&mode=pro&lang=en&page={which}", wait_until="load")
    page.wait_for_timeout(8000)
    page.evaluate("() => { window.__perf.commits = 0; window.__perf.renders = 0; window.__perf.byName = {}; window.__perf.byPage = {}; }")
    m0 = metrics(cdp)
    page.wait_for_timeout(seconds * 1000)
    m1 = metrics(cdp)
    perf = page.evaluate("window.__perf")
    d = lambda k: (m1[k] - m0[k]) / seconds
    top = sorted(perf["byName"].items(), key=lambda kv: -kv[1])[:14]
    pages = sorted(perf["byPage"].items(), key=lambda kv: -kv[1])[:8]
    print(json.dumps({
        "page": which,
        "commitsPerS": round(perf["commits"] / seconds, 1),
        "rendersPerS": round(perf["renders"] / seconds, 1),
        "scriptMsPerS": round(d("ScriptDuration") * 1000, 1),
        "taskMsPerS": round(d("TaskDuration") * 1000, 1),
        "layoutsPerS": round(d("LayoutCount"), 1),
        "styleRecalcsPerS": round(d("RecalcStyleCount"), 1),
        "top": [(k, round(v / seconds, 1)) for k, v in top],
        "byPage": [(k, round(v / seconds, 1)) for k, v in pages],
    }))
    ctx.close()


def long(browser, base, minutes):
    ctx, page, cdp = new_page(browser)
    page.goto(f"{base}/?demo&mode=pro&lang=en&page=live-run", wait_until="load")
    page.wait_for_timeout(8000)
    samples = []
    start = time.time()
    while True:
        gc(page, cdp)
        m = metrics(cdp)
        t = round(time.time() - start)
        # Typed arrays live outside the JS heap: their backing stores count too.
        usage = cdp.send("Runtime.getHeapUsage")
        backing = usage.get("backingStorageSize", 0)
        samples.append({"t": t, "heapMB": round(m["JSHeapUsedSize"] / 1e6, 2), "backingMB": round(backing / 1e6, 2), "nodes": m["Nodes"], "listeners": m["JSEventListeners"]})
        print(json.dumps(samples[-1]), flush=True)
        if t >= minutes * 60:
            break
        page.wait_for_timeout(30000)
    ctx.close()


def xray(browser, base):
    ctx, page, cdp = new_page(browser, GL_HOOK)
    page.goto(f"{base}/?demo&mode=pro&lang=en", wait_until="load")
    page.wait_for_timeout(6000)
    nav = "async (p) => { const m = await import('/src/shell/nav.ts'); m.navigate(p); }"

    def snap(label):
        gc(page, cdp)
        m = metrics(cdp)
        gl = page.evaluate("({...window.__gl})")
        info = page.evaluate("window.__caruspexXray ? window.__caruspexXray.memoryInfo() : null")
        print(json.dumps({"at": label, "gl": gl, "three": info, "heapMB": round(m["JSHeapUsedSize"] / 1e6, 2), "nodes": m["Nodes"], "listeners": m["JSEventListeners"]}), flush=True)

    page.evaluate(nav, "xray")
    page.wait_for_timeout(4000)
    page.evaluate(nav, "start")
    page.wait_for_timeout(500)
    snap("after first visit")
    for _ in range(20):
        page.evaluate(nav, "xray")
        page.wait_for_timeout(1200)
        page.evaluate(nav, "start")
        page.wait_for_timeout(400)
    snap("after 20 visits")
    page.evaluate(nav, "xray")
    page.wait_for_timeout(1500)
    theme = "async (a) => { const m = await import('/src/lib/theme.ts'); m.setAppearance(a); }"
    for i in range(20):
        page.evaluate(theme, "light" if i % 2 == 0 else "dark")
        page.wait_for_timeout(900)
    snap("after 20 rebuilds (theme)")
    ctx.close()


def xraylive(browser, base, seconds=10):
    ctx, page, cdp = new_page(browser, GL_HOOK)
    page.goto(f"{base}/?demo&mode=pro&lang=en&page=start-live", wait_until="load")
    page.wait_for_timeout(8000)
    page.evaluate("async () => { const m = await import('/src/shell/nav.ts'); m.navigate('xray'); }")
    page.wait_for_timeout(5000)
    page.evaluate("() => { window.__perf.commits = 0; window.__perf.renders = 0; window.__perf.byName = {}; for (const k in window.__glMade) window.__glMade[k] = 0; }")
    m0 = metrics(cdp)
    page.wait_for_timeout(seconds * 1000)
    m1 = metrics(cdp)
    perf = page.evaluate("window.__perf")
    made = page.evaluate("({...window.__glMade})")
    d = lambda k: (m1[k] - m0[k]) / seconds
    top = sorted(perf["byName"].items(), key=lambda kv: -kv[1])[:10]
    print(json.dumps({
        "page": "xray with live values",
        "commitsPerS": round(perf["commits"] / seconds, 1),
        "rendersPerS": round(perf["renders"] / seconds, 1),
        "scriptMsPerS": round(d("ScriptDuration") * 1000, 1),
        "taskMsPerS": round(d("TaskDuration") * 1000, 1),
        "glCreatedPerS": {k: round(v / seconds, 1) for k, v in made.items()},
        "top": [(k, round(v / seconds, 1)) for k, v in top],
    }))
    ctx.close()


def main():
    mode, base = sys.argv[1], sys.argv[2].rstrip("/")
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, args=ARGS)
        if mode == "startup":
            startup(browser, base, int(sys.argv[3]) if len(sys.argv) > 3 else 1)
        elif mode == "live":
            live(browser, base, sys.argv[3] if len(sys.argv) > 3 else "live-run")
        elif mode == "long":
            long(browser, base, float(sys.argv[3]))
        elif mode == "xray":
            xray(browser, base)
        elif mode == "xraylive":
            xraylive(browser, base)
        browser.close()


if __name__ == "__main__":
    main()
