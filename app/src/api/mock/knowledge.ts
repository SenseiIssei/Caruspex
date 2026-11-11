// Lookups into the core data that the demo can answer: fault codes, zones,
// the PID table. Catalog and VIN lookups live in areas/knowledge.ts.
import { whatToDo } from "./advice";
import { lexicon, locateDtc, lookupDtc, genericCount } from "./dtc";
import { DEFAULT_LIVE_PIDS, pidTable } from "./pids";
import { L } from "./core";
import type { PartialHandlers } from "./runtime";

export const knowledgeHandlers: PartialHandlers = {
  lookup_dtc: ({ code }) => lookupDtc(code),
  locate_dtc: ({ code }) => locateDtc(code),
  what_to_do: ({ input }) => whatToDo(input),
  dtc_lexicon: () => lexicon(),
  brand_families: () => [{ key: "", label: L("SAE/ISO (alle Marken)", "SAE/ISO (all brands)"), count: genericCount() }],
  brand_family: () => "",
  pid_table: () => pidTable(),
  default_live_pids: () => DEFAULT_LIVE_PIDS,
  pid_packs: () => [],
  obdb_packs: () => [],
};
