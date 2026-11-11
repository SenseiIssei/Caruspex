// Start-up (main.cpp): settings first, then the language and the look, so
// every page is built with the right texts. In a browser the address can
// override them for this run, like the command line options of the Qt
// version: ?lang=ar, ?theme=light, ?page=faults, ?mode=beginner, ?demo.
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { call, DEFAULT_SETTINGS } from "./api";
import { effectiveLanguage, setLanguage, t } from "./i18n";
import { setAppearance, type Appearance } from "./lib/theme";
import { emitApp } from "./lib/store";
import { App } from "./shell/App";
import { isPageId } from "./shell/pages";
import { loadGarage } from "./state/garage";
import { session, settingsStore, startSession } from "./state/session";
import "./styles/theme.css";
import "./styles/base.css";
import "./styles/shell.css";
import "./styles/widgets.css";

async function boot(): Promise<void> {
  const params = new URLSearchParams(location.search);
  const settings = await call("load_settings").catch(() => settingsStore.get().values);
  // ?mode=beginner or ?mode=pro (--mode) for this run, the setting stays.
  const mode = params.get("mode");
  const modeOverride = mode === "pro" ? true : mode === "beginner" ? false : null;
  settingsStore.set({ values: { ...DEFAULT_SETTINGS, ...settings }, loaded: true, modeOverride });

  const language = effectiveLanguage(params.get("lang") ?? settings.language);
  await setLanguage(language);
  await call("set_language", { code: language }).catch(() => undefined);
  setAppearance((params.get("theme") ?? settings.appearance) as Appearance);

  session.set({ linkText: t("caruspex::MainWindow", "Nicht verbunden") });
  startSession();
  await loadGarage();

  const root = document.getElementById("root");
  if (!root) return;
  createRoot(root).render(
    <StrictMode>
      <App startPage={params.get("page")} />
    </StrictMode>,
  );
  if (params.has("demo")) {
    setTimeout(() => emitApp("connect:demo"), 300);
    // The connection jumps to the start page; ?page follows once it has
    // settled, the way --page does in the app.
    const page = params.get("page");
    if (page) setTimeout(() => (isPageId(page) ? emitApp("navigate", page) : emitApp("dev:page", page)), 2500);
  }
}

void boot();
