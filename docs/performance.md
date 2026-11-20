# Performance

How quick and how lean Caruspex is, how that was measured and what keeps it that way. Every number here can
be measured again with the commands next to it.

## UI

The interface is React 19 in a WebView (WebView2 on Windows). Two facts shape everything below:

- **Every page stays mounted.** Pages keep listening to the backend while another page is on screen (the trip
  computer, the statistics and the X-ray view collect what happens meanwhile). So whatever a hidden page does
  per event, it does all the time. `usePageActive()` (in `app/src/lib/hooks.ts`) tells a component whether its
  page is the one on screen.
- **Live values arrive many times a second**, one `live_value` event per reading. The demo backend of the
  browser preview sends up to 150 a second, a real adapter 10 to 50.

### Rules for UI code

- **Show readings with `useLiveValues` / `useLiveRows`** (`app/src/state/liveView.ts`), not with a
  `useBackendEvent("live_value")` that sets state. They render at most once per animation frame, only when a
  shown value changed, and not at all while the page is hidden; the page catches up when it is shown.
  Keep `useBackendEvent("live_value")` for code that needs every reading (the chart, the CSV recording, the
  performance timer, the trip statistics), and let it write to refs, not state.
- **Animate outside React.** The needle of `Gauge` moves by setting four SVG attributes in an animation
  frame; React renders the gauge only when its value changes.
- **Big lists are memoised and split.** The live page builds its list of values once per tick or search, the
  table of readings is its own component with one memoised row per value. `StatTile` is memoised.
- **Hidden pages do not poll.** Timers that refresh what is on screen run only while `usePageActive()` is
  true (the "What should I do now?" advice, the live table, the X-ray labels).
- **Large data loads on the first visit.** The fault code lexicon fetches and draws its table when it is first
  opened (it was ten thousand elements at start).
- **One backend subscription per event name.** `app/src/api/hub.ts` shares one Tauri `listen()` between all
  handlers of an event, so a hundred `useBackendEvent` calls at start are about fifty round trips, not a
  hundred. A handler that throws does not stop the others.
- **three.js resources are freed or shared.** Anything removed from the X-ray scene goes through
  `disposeTree` (`app/src/xray/dispose.ts`); the packets share one geometry and one pair of materials per bus.
  The render loop stops while the page is hidden and while the window is minimised (`visibilitychange`).
- **The demo backend is not part of the app's start.** `app/src/api/index.ts` loads it with a dynamic import
  only in a plain browser; `tools/bundle_budget.py` fails the build if it, three.js or a catalog ends up in the
  entry chunk.

### Budget in the CI

`python tools/bundle_budget.py` runs after `npm --prefix app run build` in the Checks job. The budgets are
generous on purpose (entry chunk 900 KB / 300 KB gzip, its style sheet 140 KB, any later chunk 1200 KB, the
whole build 20 MB) and catch a large dependency or data file that slipped into the start, not a few
kilobytes. If a change needs more, raise the number in the script and say why in the pull request.

To see what a chunk contains, build with source maps into a scratch folder
(`npx --prefix app vite build --sourcemap --outDir <scratch>`) and attribute the bytes of
`assets/index-*.js` to its sources with any source map explorer.

### How it was measured

`tools/ui_measure.py` drives a headless Chromium with Python Playwright (installed by whoever measures, not a
dependency of the project) against the browser preview with the demo car, 1440x920:

```bash
npm --prefix app run build
npx --prefix app vite preview --port 5392            # the release build
npm --prefix app run dev                             # the development server, port 5287
py -I tools/ui_measure.py startup http://localhost:5392 4   # median of 5 starts, CPU four times slower
py -I tools/ui_measure.py live http://localhost:5392 live-run
py -I tools/ui_measure.py live http://localhost:5392 start-live
py -I tools/ui_measure.py long http://localhost:5392 10
py -I tools/ui_measure.py xray http://localhost:5287
py -I tools/ui_measure.py xraylive http://localhost:5287
```

Renders are counted through a stand-in for the React DevTools hook (a component counts when it did work in a
commit), so the release build is measured as shipped; against the development server the counts come with
component names and the page they belong to. CPU time is Chromium's `ScriptDuration` and `TaskDuration`,
memory the JS heap after a forced garbage collection plus the backing stores of typed arrays. The X-ray counts
come from wrapped WebGL calls (buffers, textures, programs alive and created) and, on the development server,
`renderer.info` through `window.__caruspexXray`, which only the development build sets.

### Numbers (November 2026)

Before and after the work of November 2026 (branch `perf-ui`). Browser numbers are from the release build in
headless Chromium with the demo backend (about 150 readings a second); "app" numbers from the release app on
Windows 11 with WebView2, the demo car and `--page live-run` or `--page start-live`, measured through
WebView2's DevTools port.

| What | Before | After |
|---|---|---|
| Entry chunk (`vite build`) | 1217 kB, 364 kB gzip | 732 kB, 234 kB gzip |
| Demo backend | inside the entry chunk | own chunk (488 kB), never loaded by the app |
| Start, browser, CPU 4x slower: DOM content loaded | 169 to 245 ms | 129 to 142 ms |
| Start, browser, CPU 4x slower: last long task ends | 967 to 1169 ms | 783 to 950 ms |
| Start, browser, CPU 4x slower: total blocking time | 50 to 104 ms | 6 to 40 ms |
| Start, app: DOM content loaded / first paint | 49 / 220 ms | 38 / 216 ms |
| DOM nodes after start | 25 256 | 6 751 (the lexicon alone was 10 113 elements) |
| JS event listeners after start | 2 350 | 671 |
| JS heap after start (after GC) | 11.6 MB | 8.9 MB |
| Live page, browser: React commits / component renders per s | 76 to 88 / 1166 to 1198 | 28 / 369 to 375 |
| Live page, browser: script time per s | 33 to 37 ms | 15 to 16 ms |
| Start page with gauges, browser: commits / renders per s | 72 to 74 / 1531 to 1549 | 49 / 724 to 726 |
| Live page, app: script / all main thread work per s | 69 / 230 ms | 17 / 191 ms |
| Start page with gauges, app: script / all main thread work per s | 75 / 505 ms | 26 / 392 ms |
| Live page, app: JS heap after GC / WebView renderer private memory | 23.7 / 227 MB | 10.5 / 180 MB |
| 10 min live session with recording, browser: heap plus typed arrays | +4.6 MB (17.3 to 21.9) | +1.5 MB (12.1 to 13.6) |
| 10 min live session: DOM nodes, listeners | flat | flat |
| X-ray page opened 20 times, rebuilt 20 times | no leak (WebGL buffers 267, textures 8, programs 11 before and after) | same, `renderer.info`: 76 geometries, 4 textures, 11 programs throughout |
| X-ray page with live values: WebGL objects created per s | 3.6 buffers, 0.6 textures, a shader program every 10 s | 0 |

Of the renders left, the hidden throttle page (`ThrottlePage.tsx`, which sets state on every reading) makes
about 300 a second on the live page and 600 on the start page; without it the live page is at about 70
renders a second and the start page at about 120. The main thread work that is left on the start page is
mostly layout and paint of the six needles, which move sixty times a second as before.

Screenshots of the live page, the start page with its gauges and the X-ray view (with packets in flight) look
the same before and after.

### Still open

- `ThrottlePage.tsx` should show its readings with `useLiveValues` like the other pages.
- The demo backend, its fault code table and the core catalogs it uses (about 9 MB of the 14 MB build) are
  never loaded by the app but are still built into its program file; a separate build of the interface for
  the app would leave them out.
- `XrayPage` keeps the layout code of the 3D view (about 110 kB) in the entry chunk, because the page
  collects scan results while hidden; moving the layout into the lazy chunk needs the page split in two.
- Every Tauri event reaches the WebView as a script of its own. A batched live event from the backend would
  cut that per reading cost; the interface keeps handling `live_value` either way.
