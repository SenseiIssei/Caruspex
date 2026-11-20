# Bezpečnost a aktualizace

Tato stránka shrnuje bezpečnostní informace o Caruspexu: k čemu program slouží, jaká rizika je dobré znát, jak
fungují aktualizace a jak ho odstranit. Odpovídá informacím pro uživatele, které požaduje evropský akt o
kybernetické odolnosti (Cyber Resilience Act, příloha II), pokud se týkají svobodného nekomerčního programu.

## Program

- **Název**: Caruspex, aplikace `caruspex` a nástroj příkazové řádky `caruspex_cli`, pro Windows, Linux a macOS.
  Verze je v názvu každého stažení a v seznamu změn každého vydání.
- **Vydává**: projekt Caruspex na [GitHubu](https://github.com/SenseiIssei/Caruspex), jako svobodný software pod
  licencí MIT. Dotazy a chyby patří do [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Účel**: čtení a vysvětlení diagnostických dat aut, motocyklů a nákladních vozidel přes vlastní adaptér na
  vlastním počítači. Bez účtu, bez cloudu a bez sledování ([Soukromí](privacy.md)).
- **Bezpečnostní vlastnosti**: čtení v autě nikdy nic nemění a vše, co auto přiměje něco udělat, se nejdřív zeptá.
  Aktualizace se nainstaluje jen tehdy, když její podpis odpovídá klíči zabudovanému v aplikaci. Caruspex neotevírá
  žádný síťový port a nespouští kód ze souborů ani z internetu.

## Nahlášení zranitelnosti {#report}

Bezpečnostní problémy hlaste soukromě přes
[formulář nového bezpečnostního hlášení](https://github.com/SenseiIssei/Caruspex/security/advisories/new), ne ve
veřejném issue. [Bezpečnostní zásady](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) popisují, co
následuje. Opravené zranitelnosti se zveřejňují jako
[bezpečnostní upozornění](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Rizika, která je dobré znát {#risks}

- Caruspex stahujte jen ze [stránky vydání](https://github.com/SenseiIssei/Caruspex/releases). Kopie odjinud mohou
  být upravené.
- Záznamy, balíčky PID a 3D modely otevírejte jen od lidí, kterým věříte. Nemohou spustit kód, ale poškozený soubor
  může Caruspex zpomalit nebo zavřít.
- Mazání chybových kódů, testy akčních členů a základní nastavení provádějte jen se stojícím autem na bezpečném místě.
- Mnoho adaptérů Bluetooth a Wi-Fi nemá PIN nebo má otevřenou síť: připojit se může kdokoli poblíž. Po práci adaptér
  vytáhněte.
- Kabely ENET, DoIP a adaptéry Wi-Fi nic nešifrují: používejte je přímo nebo jen ve vlastní síti.
- Ovladače J2534 jsou programy svých výrobců a běží s vašimi právy. Instalujte je jen od výrobce.

## Aktualizace a podpora {#support}

- Při spuštění se Caruspex zeptá GitHubu, zda existuje novější verze, a zobrazí ji. Instalace je jedno kliknutí a
  aplikace nainstaluje jen balíček se správným podpisem. AppImage a aplikace pro macOS se aktualizují samy; balíček
  .deb a aplikace pro Android dostanou stránku ke stažení.
- Hledání i automatická instalace jsou přepínače v nastavení v části **Okno a jazyk**. Bez hledání se občas podívejte
  na stránku vydání.
- Bezpečnostní opravy jsou jen v nejnovějším vydání. Starší verze samostatné opravy nedostávají, proto Caruspex
  udržujte aktuální.
- Caruspex se udržuje ve volném čase a nemá pevné datum konce. Pokud údržba skončí, repozitář a poslední vydání to
  oznámí předem.

## Kontrola staženého souboru {#verify}

Každé vydání má soubor `SHA256SUMS` s kontrolním součtem každého souboru a záznam podepsaný GitHubem o tom, jak byly
soubory sestaveny. Se soubory v jedné složce:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Bez těchto nástrojů ukáže `Get-FileHash <soubor>` v PowerShellu (Windows) nebo `shasum -a 256 <soubor>` (macOS)
kontrolní součet k porovnání s řádkem v `SHA256SUMS`.

## Odstranění Caruspexu a vlastních dat {#remove}

1. Odinstalujte program: ve Windows v Nastavení, Aplikace; v macOS přetáhněte Caruspex ze složky Aplikace; v Linuxu
   smažte AppImage nebo použijte `sudo apt remove caruspex`.
2. Smažte data: tlačítko **Otevřít umístění úložiště** v nastavení ukáže složky s nastavením, garáží a hlášeními o
   pádech. Smažte je. Záznamy a zprávy uložené vámi jsou tam, kam jste je uložili.

## Pro vývojáře a firmy {#integrators}

- Každé vydání má pro každou platformu soupis softwarových komponent `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX) s každou knihovnou a její verzí.
- Posouzení rizik, seznam požadavků a oblast působnosti Cyber Resilience Act pro Caruspex jsou v
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex není uváděn na trh jako produkt ve smyslu Cyber Resilience Act, proto nemá EU prohlášení o shodě ani
  označení CE.
