// Small helpers the garage page and its tabs share.
import { localeDate, t } from "../../i18n";
import { parseIso } from "../../state/garage";

export const Q = "QObject";
// QDialogButtonBox: the cancel text of the pages that have one.
export const cancelText = () => t("caruspex::FaultsPage", "Abbrechen");

export const shortDate = (iso: string) => {
  const d = parseIso(iso);
  return d ? localeDate(d, { dateStyle: "short" }) : "";
};
export const shortDateTime = (iso: string) => {
  const d = parseIso(iso);
  return d ? localeDate(d, { dateStyle: "short", timeStyle: "short" }) : "";
};
