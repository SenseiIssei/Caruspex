# User information (Annex II)

Status: 21 November 2026, Caruspex 0.17. Annex II of the Cyber Resilience Act (Regulation (EU) 2024/2847) lists
the information that must accompany a product with digital elements (Article 13(18)). Caruspex is currently
outside the scope of the CRA ([scope.md](scope.md)); the information is published anyway, for users on the
**Security and updates** page of the documentation website in all 18 languages
(`docs/site/content/<language>/security.md`, online at
<https://senseiissei.github.io/Caruspex/en/security.html>), and for developers in [SECURITY.md](../../SECURITY.md).

| Annex II point | Where it is answered | Note |
|---|---|---|
| 1 Manufacturer name, postal address, e-mail or other digital contact, website | security page, "The program": the Caruspex project on GitHub, the issues | No postal or e-mail address: Caruspex is not placed on the market, and the project publishes no personal contact details. A manufacturer would have to add them (Article 13(16)). |
| 2 Single point of contact for vulnerabilities, coordinated vulnerability disclosure policy | security page, "Reporting a vulnerability"; [SECURITY.md](../../SECURITY.md); `/.well-known/security.txt` of the website | GitHub's private vulnerability reporting. A manufacturer's single point of contact must not rely only on automated tools (Article 13(17)). |
| 3 Name, type, unique identification | security page, "The program"; the version in every file name and in `latest.json` | |
| 4 Intended purpose, security environment, essential functions, security properties | security page, "The program"; [risk-assessment.md](risk-assessment.md) | |
| 5 Known or foreseeable circumstances that may lead to significant cybersecurity risks | security page, "Risks to know about"; [risk-assessment.md](risk-assessment.md) | |
| 6 Address of the EU declaration of conformity | security page, "For developers and companies" | Not applicable: no declaration is drawn up. |
| 7 Type of technical security support, end of the support period | security page, "Updates and support"; [support.md](support.md); SECURITY.md, "Supported versions" | No fixed end date (see support.md). |
| 8(a) Secure commissioning and use | security page, "Risks to know about", "Checking a download"; [install page](../site/content/en/install.md) | |
| 8(b) How changes to the product can affect the security of data | security page, "Risks to know about" (copies from elsewhere, files from strangers) | |
| 8(c) How to install security updates | security page, "Updates and support" | |
| 8(d) Secure decommissioning, removal of user data | security page, "Removing Caruspex and your data"; [privacy page](../site/content/en/privacy.md) | |
| 8(e) How to turn off automatic installation of security updates | security page, "Updates and support": the switches under **Window and language** in the settings | Automatic installation is off by default today; the check is on. |
| 8(f) Information for integrators | security page, "For developers and companies"; this folder; the SBOMs | |
| 9 Where the SBOM can be found | security page, "For developers and companies"; SECURITY.md, "Checking a download" | `Caruspex-<version>-sbom-<platform>.cdx.json` on every release. |

The information must be kept available for at least 10 years or the support period, whichever is longer (Article
13(18)); the website and this folder are part of the repository and its history.
