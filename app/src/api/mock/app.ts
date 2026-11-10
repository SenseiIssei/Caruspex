// App level commands for the browser preview: settings and the garage live
// in localStorage, files go through the browser's download and file picker.
import type { AppSettings, FileFilter, GarageCar, OpenedTextFile } from "../types";
import { DEFAULT_SETTINGS } from "../types";
import { setCoreLanguage } from "./core";
import { applyPollInterval } from "./session";
import type { PartialHandlers } from "./runtime";

const SETTINGS_KEY = "caruspex.settings";
const GARAGE_KEY = "caruspex.garage";

function readJson<T>(key: string, fallback: T): T {
  try {
    const text = localStorage.getItem(key);
    return text ? (JSON.parse(text) as T) : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Private windows may refuse storage; the preview still works.
  }
}

function download(name: string, content: string | Blob, type = "text/plain"): void {
  const blob = content instanceof Blob ? content : new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

function pickFile(filters: FileFilter[]): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = filters.flatMap((f) => f.extensions.map((e) => `.${e}`)).join(",");
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}

// Prints an HTML document through a hidden frame; the browser offers
// "save as PDF" there.
function printHtml(html: string): void {
  const frame = document.createElement("iframe");
  frame.style.position = "fixed";
  frame.style.width = "0";
  frame.style.height = "0";
  frame.style.border = "0";
  document.body.appendChild(frame);
  const doc = frame.contentDocument;
  if (!doc) return;
  doc.open();
  doc.write(html);
  doc.close();
  setTimeout(() => {
    frame.contentWindow?.focus();
    frame.contentWindow?.print();
    setTimeout(() => frame.remove(), 1000);
  }, 250);
}

export const appHandlers: PartialHandlers = {
  app_info: () => ({ version: "0.9.0", hasBle: true, platform: "browser", documentsDir: "~/Dokumente" }),
  set_language: async ({ code }) => {
    await setCoreLanguage(code);
  },
  // ?update=1 in the address shows the update button.
  check_for_update: () =>
    new URLSearchParams(location.search).has("update")
      ? { version: "0.9.1", url: "https://github.com/SenseiIssei/Caruspex/releases/latest" }
      : null,
  open_url: ({ url }) => {
    window.open(url, "_blank", "noopener");
  },
  load_settings: () => ({ ...DEFAULT_SETTINGS, ...readJson<Partial<AppSettings>>(SETTINGS_KEY, {}) }),
  save_settings: ({ values }) => {
    writeJson(SETTINGS_KEY, values);
    applyPollInterval(values.pollIntervalMs);
  },
  // The browser preview has no log file.
  write_log: () => undefined,
  save_text_file: ({ suggestedName, content }) => {
    download(suggestedName, content);
    return suggestedName;
  },
  save_pdf: ({ html }) => {
    printHtml(html);
    return null;
  },
  open_text_file: async ({ filters }): Promise<OpenedTextFile | null> => {
    const file = await pickFile(filters);
    if (!file) return null;
    return { path: file.name, content: await file.text() };
  },
  pick_save_path: ({ suggestedName }) => `~/Dokumente/${suggestedName}`,
  pick_open_path: async ({ filters }) => {
    const file = await pickFile(filters);
    return file ? file.name : null;
  },
  garage_load: () => readJson<GarageCar[]>(GARAGE_KEY, []),
  garage_save: ({ cars }) => writeJson(GARAGE_KEY, cars),
};
