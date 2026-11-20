# Cyber Resilience Act: scope assessment

Status: 21 November 2026, Caruspex 0.17. This is the project's own reading of the law, not legal advice. Sources:

- Regulation (EU) 2024/2847 (Cyber Resilience Act, "CRA"), OJ L, 20.11.2024,
  [ELI](http://data.europa.eu/eli/reg/2024/2847/oj). Article and recital numbers below refer to it.
- Commission guidance on the application of the CRA, C/2026/6013, OJ C, 17.11.2026,
  [ELI](http://data.europa.eu/eli/C/2026/6013/oj), section 3 (not binding, point 8 of the guidance).
- Commission CRA FAQ, first published 3 December 2025, updated 4 September 2026
  ([page](https://digital-strategy.ec.europa.eu/en/library/cyber-resilience-act-implementation-frequently-asked-questions)).
- Directive (EU) 2024/2853 on liability for defective products ("PLD"), OJ L, 18.11.2024,
  [ELI](http://data.europa.eu/eli/dir/2024/2853/oj).

## Result

Caruspex is free and open-source software that is not monetised. As published today it is **not placed on
the market** in the sense of the CRA, so the obligations of manufacturers (Articles 13 and 14) do not apply to
it. The project is run by a natural person, so it is **not an open-source software steward** either (only a
legal person can be one, Article 3(14)).

The project still follows the manufacturer requirements where that is cheap and useful (SBOM, vulnerability
handling, signed updates, user information), so that a company integrating Caruspex can do its due diligence
(Article 13(5)) and so that little is missing if the situation below ever changes.

## Why

1. **Caruspex is a product with digital elements.** It is software (Article 3(1)) whose intended use includes
   data connections to a device or network: the OBD adapter over USB, Bluetooth, Wi-Fi or Ethernet, and the
   update check (Article 2(1), Article 3(8) and (9)). The exclusions of Article 2(2) to (7) do not fit:
   Regulation (EU) 2019/2144 (Article 2(2)(c)) covers the type approval of vehicles and their systems, not a
   separate diagnostics program that the owner installs on a computer.
2. **The CRA only covers products "made available on the market"**, which means supplied "in the course of a
   commercial activity, whether in return for payment or free of charge" (Article 2(1), Article 3(22),
   recital 15).
3. **Free and open-source software that its manufacturer does not monetise is not supplied in the course of a
   commercial activity** (recital 18; guidance points 42 and 50 to 62). Caruspex is MIT licensed with its source
   code public on GitHub, so it is FOSS in the sense of Article 3(48) (guidance points 44 to 46).
4. **Caruspex is not monetised** in any of the ways recital 15 and the guidance name:
   - no price for the program, the binaries or the updates (guidance point 51, examples 21 and 22);
   - no paid support, no paid edition, no platform through which other services are sold (guidance points 54
     to 57, examples 14, 15, 17);
   - no processing of personal data as a condition for use: there is no account, no telemetry, and the only
     automatic network request is the update check, which sends nothing about the user or the car and can be
     switched off ([privacy page](../site/content/en/privacy.md), `app/src-tauri/src/update.rs`,
     guidance point 54, example 16);
   - no donations are collected at the moment. Donations without the intention of making a profit would not
     change the result either (recital 15, guidance points 60 and 61, example 20), as long as downloads,
     updates and security fixes are never tied to a donation (guidance point 62).
5. **Regular releases, a release workflow and contributions from others do not make it commercial** (recital 18:
   "the mere presence of regular releases should not in itself lead to the conclusion..."; guidance example 13
   for contributors). Hosting on GitHub and package managers is not making available on the market
   (recital 20).
6. **A natural person publishing a free version is outside the CRA** (guidance point 53; example 27 for an
   individual developer whose software companies integrate).

## Dates that would matter if Caruspex were in scope

| What | From | Where |
|---|---|---|
| Reporting of actively exploited vulnerabilities and severe incidents (24 h, 72 h, final report) via ENISA's single reporting platform | 11 September 2026 | Article 14, Article 71(2) second subparagraph |
| Article 14 also for products placed on the market before 11 December 2027 | 11 September 2026 | Article 69(3) |
| Conformity assessment bodies (Chapter IV) | 11 June 2026 | Article 71(2) |
| Everything else: essential requirements, technical documentation, conformity assessment, CE marking, obligations of stewards | 11 December 2027 | Article 71(2) first subparagraph |

The Commission FAQ ("When does the CRA start applying?") and its reporting page confirm these dates; the single
reporting platform has been operating since 11 September 2026.

## What would change it

Caruspex would become a product placed on the market, and whoever publishes it a **manufacturer** (Article
3(13)), as soon as one of these happens:

- a price for the program, for prebuilt packages, for updates or for "pro" features;
- a paid edition or paid support that is bundled with access to the software (guidance points 56 and 57);
- the app is used to sell other services (a shop, paid cloud functions, advertising) or requires personal data
  beyond security and compatibility (recital 15);
- downloads, updates or fixes only for donors (guidance point 62);
- a company takes over the project and supplies it as part of its business.

A company that **integrates** Caruspex into its own monetised product is the manufacturer of that product, not
of Caruspex. It must exercise due diligence on Caruspex as a component (Article 13(5); guidance points 86 to 88)
and report vulnerabilities it finds in Caruspex to this project (Article 13(6)). The SBOMs, the vulnerability
process in [SECURITY.md](../../SECURITY.md) and these documents are meant to make that easy. A company that
substantially modifies Caruspex and makes it available on the market becomes the manufacturer of the modified
version (Article 22).

If Caruspex became a product on the market, it would be in the default category: its core functionality is
vehicle diagnostics, none of the categories of Annex III or Annex IV, and the embedded web view of the interface
does not change that (Article 7(1), second sentence). The manufacturer would have to:

- meet the essential requirements of Annex I part I and the vulnerability handling of Annex I part II (Article
  13(1), (8)); the gaps are listed in [requirements.md](requirements.md);
- keep the cybersecurity risk assessment documented and current ([risk-assessment.md](risk-assessment.md),
  Article 13(2) to (4));
- set a support period of at least five years unless the product is used for a shorter time, and state its end
  (month and year) at the time of download (Article 13(8), (19)); see [support.md](support.md);
- keep each security update available for at least 10 years (Article 13(9));
- draw up the technical documentation of Annex VII (Article 31), carry out the internal control procedure of
  Annex VIII module A (Article 32(1)(a)), draw up the EU declaration of conformity (Article 28, Annex V) and
  affix the CE marking on the declaration or the download website (Article 30(1));
- name the manufacturer with a postal or e-mail address and a single point of contact that does not rely only
  on automated tools (Article 13(16), (17));
- ship the user information of Annex II ([user-information.md](user-information.md), Article 13(18));
- report actively exploited vulnerabilities and severe incidents within 24 hours, 72 hours and with a final
  report (Article 14); the checklist is in [SECURITY.md](../../SECURITY.md).

Fines for breaches of Annex I, Article 13 or Article 14 reach up to EUR 15 million or 2.5 % of the worldwide
annual turnover (Article 64(2)); stewards are exempt from fines (Article 64(10)(b)).

## Product liability (Directive (EU) 2024/2853)

The new Product Liability Directive applies to products placed on the market or put into service after
9 December 2026 and treats software as a product (Article 2(1), Article 4(1)). It does **not** apply to "free and
open-source software that is developed or supplied outside the course of a commercial activity" (Article 2(2),
recital 14). Recital 15 adds that when such software is integrated as a component into a product placed on the
market, the integrating manufacturer can be held liable, not the developer of the free software. The same test as
in the CRA therefore decides: as long as Caruspex is not monetised, the PLD does not apply to its developers. The
Commission FAQ notes that under the PLD a manufacturer stays liable for defects introduced by an update or caused
by a missing update, which is one more reason for signed, timely updates. The liability notices of the MIT
licence are a separate matter.
