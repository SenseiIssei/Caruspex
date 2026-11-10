// The start options of the command line that need the interface (main.cpp):
// the page of --page once a connection has settled (a connection jumps to
// the start page first), and --report, whose document only the interface can
// write. The pages and the report are imported lazily: they import the API
// themselves, and a static import here would be a cycle at start.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Events } from "./contract";

export function listenToCommandLine(): void {
  void listen<Events["cli_page"]>("cli_page", async ({ payload }) => {
    const [{ isPageId }, { navigate }, { emitApp }] = await Promise.all([
      import("../shell/pages"),
      import("../shell/nav"),
      import("../lib/store"),
    ]);
    if (isPageId(payload.page)) navigate(payload.page);
    else emitApp("dev:page", payload.page);
    if (payload.page === "log") emitApp("log:open");
  });

  void listen<Events["cli_report"]>("cli_report", async ({ payload }) => {
    const { collectReportExtras, collectReportState, reportHtml } = await import("../report/report");
    const html = reportHtml(await collectReportState(), collectReportExtras());
    await invoke("write_report", { path: payload.path, html });
  });
}
