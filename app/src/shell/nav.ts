// Which page is showing, and the report preview on top of it.
import { createStore, emitApp, useStore } from "../lib/store";
import type { PageId } from "./pages";

export const navStore = createStore<{ page: PageId; reportOpen: boolean; garageReportVin: string }>({
  page: "start",
  reportOpen: false,
  garageReportVin: "",
});

export function navigate(page: PageId): void {
  if (navStore.get().page !== page) navStore.set({ page });
  emitApp("navigated", page);
}

export function useCurrentPage(): PageId {
  return useStore(navStore, (s) => s.page);
}

// The inspection report of the connected car (MainWindow::showReport).
export function openReport(): void {
  navStore.set({ reportOpen: true, garageReportVin: "" });
}

// The file of one garage car (GaragePage's report button).
export function openGarageReport(vin: string): void {
  navStore.set({ reportOpen: true, garageReportVin: vin });
}

export function closeReport(): void {
  navStore.set({ reportOpen: false });
}
