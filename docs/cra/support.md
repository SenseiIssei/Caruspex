# Support period and end of support

Status: 21 November 2026, Caruspex 0.17.

## What users can expect today

- **Only the newest release gets fixes.** A security fix is published as a new release, which the built-in
  updater offers to every installation of Windows, Linux (AppImage) and macOS. Older versions do not get separate
  fixes; updating to the newest release is free and needs nothing else on the computer.
- **No fixed end date.** Caruspex is a free, non-commercial project maintained in spare time. There is no
  contract and no guaranteed support period. Vulnerabilities are handled as described in
  [SECURITY.md](../../SECURITY.md) for as long as the project is maintained.
- **Announced end.** If the project stops being maintained, this will be announced in the repository, in the
  release notes of a last release and on the documentation website, at least 90 days ahead where possible. The
  repository will then be archived; the releases, the SBOMs and the advisories stay available on GitHub.
- **Old releases** remain downloadable on the releases page for reference. They are unsupported: they may contain
  known vulnerabilities (CRA Article 13(11) asks to say exactly this when old versions stay available).

## What the CRA would require

Caruspex is outside the scope of the CRA ([scope.md](scope.md)). If it were placed on the market, the manufacturer
would have to:

- set the support period to the time the product is expected to be in use, at least five years unless it is used
  for a shorter time (Article 13(8)); for a diagnostics program for cars that are kept for many years, five years
  is the minimum that would be defensible;
- state the end of the support period, at least month and year, clearly at the time of download, and show a
  notice in the app when it is reached where technically feasible (Article 13(19));
- keep each security update available for at least 10 years after it was issued or for the rest of the support
  period, whichever is longer (Article 13(9));
- may handle vulnerabilities only in the newest version if users of older versions can update to it free of
  charge and without new hardware or software costs (Article 13(10)), which matches the practice above.

## Information used for this statement (Annex VII point 4)

- Cars and adapters are kept for many years; the program is expected to be used as long as people own OBD-II
  cars.
- The program depends on the operating systems' web views (WebView2, WebKitGTK, WKWebView) and on tauri; their
  support ends would limit how long a release can be supported.
- The project has no income and one maintainer, so it does not promise a period it cannot guarantee.
