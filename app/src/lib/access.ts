// Accessibility settings that change the whole window: the high contrast
// palette (styles/theme.css, data-contrast="high") and the size of
// everything, which the backend applies as the zoom of the web view so
// charts and the X-ray view keep their mouse positions.
import { call } from "../api";

// The translation context of texts that only screen readers and keyboard
// users meet (names of icon buttons, step buttons).
export const ACCESS = "caruspex::Access";

// The steps offered in the settings, in percent.
export const FONT_SCALES = [90, 100, 115, 130, 150] as const;

export function setHighContrast(on: boolean): void {
  if (typeof document === "undefined") return;
  if (on) document.documentElement.dataset.contrast = "high";
  else delete document.documentElement.dataset.contrast;
}

export function setFontScale(percent: number): void {
  const scale = Math.min(160, Math.max(80, Number.isFinite(percent) ? percent : 100)) / 100;
  void call("set_zoom", { scale }).catch(() => undefined);
}
