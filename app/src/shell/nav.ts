// Which page is showing, and the report preview on top of it.
import { createStore, emitApp, useStore } from "../lib/store";
import type { PageId } from "./pages";

// usedCarReport: the preview shows the document of the used car check.
export const navStore = createStore<{ page: PageId; reportOpen: boolean; garageReportVin: string; usedCarReport: boolean }>({
  page: "start",
  reportOpen: false,
  garageReportVin: "",
  usedCarReport: false,
});

// The pages besides the first one mount just after the first picture (see
// PageHost in App.tsx); this resolves once all of them listen.
let pagesDone: () => void = () => undefined;
const pagesReady = new Promise<void>((resolve) => (pagesDone = resolve));

export function markPagesMounted(): void {
  pagesDone();
}

export function pagesMounted(): Promise<void> {
  return pagesReady;
}

export function navigate(page: PageId): void {
  if (navStore.get().page !== page) navStore.set({ page });
  emitApp("navigated", page);
}

export function useCurrentPage(): PageId {
  return useStore(navStore, (s) => s.page);
}

// The inspection report of the connected car (MainWindow::showReport).
export function openReport(): void {
  navStore.set({ reportOpen: true, garageReportVin: "", usedCarReport: false });
}

// The file of one garage car (GaragePage's report button).
export function openGarageReport(vin: string): void {
  navStore.set({ reportOpen: true, garageReportVin: vin, usedCarReport: false });
}

// The document of the last used car check, for the purchase.
export function openUsedCarReport(): void {
  navStore.set({ reportOpen: true, garageReportVin: "", usedCarReport: true });
}

export function closeReport(): void {
  navStore.set({ reportOpen: false });
}
