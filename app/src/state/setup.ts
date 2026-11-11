// The first-start assistant: open on the first start of a new install,
// reopened from the settings and the connect page. Closing it in any way
// (finished or skipped) marks it as shown, so it does not come back by
// itself.
import { createStore } from "../lib/store";
import { applySettings, settingsStore } from "./session";

// preset: the step to start on and a car to search for (--page setup-socket).
export interface SetupPreset {
  step: number;
  query?: string;
}

export const setupStore = createStore<{ open: boolean; preset: SetupPreset | null }>({ open: false, preset: null });

export function openSetup(preset: SetupPreset | null = null): void {
  setupStore.set({ open: true, preset });
}

export function closeSetup(): void {
  setupStore.set({ open: false, preset: null });
  const values = settingsStore.get().values;
  if (!values.setupDone) void applySettings({ ...values, setupDone: true });
}
