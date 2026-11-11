// The last used car check, shared by the page, the report preview and the
// --report of the command line. The page fills it from used_car_result.
import type { UsedCarRow } from "../api";
import { createStore } from "../lib/store";

export const usedCarStore = createStore<{ result: UsedCarRow | null; when: number }>({
  result: null,
  when: 0,
});
