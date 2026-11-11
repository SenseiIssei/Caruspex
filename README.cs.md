# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · **Čeština** · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Open-source diagnostika vozidel pro Windows, Linux a macOS, napsaná v Rustu s rozhraním na Tauri 2.
Jeden kabel nebo adaptér, jedna aplikace: čtení a mazání chybových kódů, sledování živých dat ze senzorů,
kontrola připravenosti na emisní test a komunikace s řídicími jednotkami VW/Audi na úrovni jednotlivých bajtů přes kabel K-Line.

Jméno pochází od *haruspika*, římského věštce, který četl znamení z vnitřností. Caruspex čte ta z auta.

Aplikace mluví 18 jazyky, rozhraní i každý text chybového kódu: anglicky, německy, španělsky,
francouzsky, portugalsky, italsky, nizozemsky, polsky, česky, rusky, ukrajinsky, turecky, arabsky (zprava doleva), čínsky,
japonsky, korejsky, hindsky a indonésky. Řídí se jazykem systému, v nastavení ho lze přepnout nebo
vynutit pro jedno spuštění přes `--language fr` a podobně. Jak se překlady vytvářejí a kontrolují, popisuje
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Přehled s živými ukazateli](docs/screenshot-overview.png)

## Hlavní přednosti

- **Dva kompletní diagnostické zásobníky**: standardní OBD-II/EOBD přes jakýkoli adaptér ELM327 a diagnostika
  výrobce VW/Audi (KWP1281 a KWP2000) přes surový kabel KKL K-Line.
- **Protokoly napsané od nuly**: KWP1281 potvrzuje každý jednotlivý bajt jeho doplňkem do jedné,
  což firmware ELM327 neumí. Caruspex ovládá K-Line sám, včetně probouzecí adresy o 5 baudech,
  odesílané bit po bitu přepínáním signálu break sériového portu s časováním podle termínů, takže
  kolísání plánovače operačního systému se v průběhu deseti bitů nesčítá.
- **Automatické rozpoznání protokolu**: po probuzení dva klíčové bajty jednotky prozradí, zda mluví
  KWP1281, nebo KWP2000 (ISO 14230); Caruspex se podle toho přepne.
- **Modulární workspace v Rustu**: jeden crate na oblast (spojení s adaptéry, protokoly, síť a CAN, vozidla, data,
  záznamy, analýza, demo auto, nástroj příkazového řádku) a nad tím aplikace Tauri. Jádro o rozhraní nic neví,
  takže aplikace i nástroj příkazového řádku běží na úplně stejném kódu.
- **Testováno bez auta**: simulované řídicí jednotky (`crates/caruspex-protocols/tests/common/`) napodobují
  navazování spojení bajt po bajtu, falešné brány zastupují ENET a DoIP, falešný ovladač J2534 pokrývá cestu PassThru
  a simulátor ELM327 v Pythonu umožňuje spuštění aplikace od začátku do konce.
- **Jedna sada překladů**: jádro a rozhraní sdílejí stejné soubory překladů v
  `translations/<code>/`, v 18 jazycích. Jádro hledá své texty podle anglického textu, rozhraní z týchž souborů
  sestavuje své katalogy.
- **Pozná každé auto**: dekodér VIN s 365 kódy výrobců a katalog více než 2 100
  generací modelů od roku 1980 ukazují, které auto je připojeno a co s ním funguje.
- **Vestavěné demo auto**: ELM327 ve stejném procesu s motorem a převodovkou na CAN, takže vše
  lze vyzkoušet bez hardwaru.
- **Generovaná data, ne psaná ručně**: asi 1 700 chybových kódů, mnoho z nich vypočtených z pravidelných rodin kódů
  (na válec, na řadu válců, na lambda sondu) skriptem `tools/gen_dtc.py`.

## Funkce

### Zná auto
- Po připojení Caruspex přečte VIN a řekne, o které auto jde: značka, model, generace, modelový rok.
- Katalog více než 2 100 generací modelů od 94 značek, od roku 1980 do dneška, včetně značek, které už neexistují.
- U každé generace: jaký diagnostický konektor, kde sedí, jaký protokol a zda bude fungovat OBD-II, kabel KKL, nebo jen vlastní nástroje výrobce.
- Kontrola VIN pro libovolné VIN: výrobce, země, modelový rok, kontrolní číslice, kód modelu.
- Garáž, která pozná každé auto podle VIN a uchovává historii jeho kontrol a stavů tachometru.

![Vozidla](docs/screenshot-vehicles.png)

### Kontrola zdraví jedním tlačítkem
- Přečte paměť závad, monitory připravenosti a testy součástí a pak půl minuty sleduje směs, teploty, lambda sondy a nabíjecí napětí.
- Skóre od 0 do 100, u něhož je každý ztracený bod vysvětlen srozumitelně, i s dalším krokem.
- Připraveno na emisní test? Který monitor je ještě otevřený a jak jet, aby se dokončil.
- Zpráva o kontrole jako PDF, k vytištění nebo pro servis.

![Kontrola zdraví](docs/screenshot-health.png)

### Paměť závad
- Uložené, čekající a trvalé kódy; u více řídicích jednotek každý kód ukazuje, kdo ho nahlásil (motor, převodovka).
- Asi 1 400 standardních kódů (P, C, B, U) plus kódy výrobců pro VAG, Ford, GM, Toyotu, Hondu, Nissan, Hyundai/Kii, Subaru a další. Kód P1 se čte tak, jak ho chápe rozpoznaná značka.
- Semafor u každého kódu a otázka, kterou si každý klade jako první: mohu jet dál?
- Obvyklé příčiny, věci, které si můžete zkontrolovat sami, a hrubá cenová třída.
- Auto v průhledném zobrazení ukazuje, kde postižená součást sedí.
- Mazání s jasným varováním a automatickým novým načtením; export do CSV.

![Paměť závad](docs/screenshot-faults.png)

### Co mám teď dělat?
- Jedna stránka, která seřadí vše, co Caruspex našel, do tří kroků: zastavit hned, brzy do servisu, počká.
- U každého bodu jedna jednoduchá věta proč, orientační rozmezí ceny a poznámka k ukázání v servisu (kód, řídicí jednotka, oblast, obvyklé příčiny).
- OBD nevidí, zda kontrolka motoru bliká nebo svítí červená výstraha: ptají se na to dvě zaškrtávací políčka a blikající kontrolka při vynechávání zapalování přesune bod do "zastavit hned".

![Co mám teď dělat](docs/screenshot-todo.png)

### Zkontrolovat ojeté auto
- Pro prohlídku před koupí: sedí VIN v autě s doklady a katalogem, shoduje se stav kilometrů mezi řídicími jednotkami, byly závady smazány těsně předtím (vzdálenost a čas od smazání)?
- Monitory připravenosti, uložené závady, kontrola zdraví a u elektromobilů stav baterie.
- Semafor, otázky pro prodejce a PDF pro nákup.

![Zkontrolovat ojeté auto](docs/screenshot-used-car.png)

### Rentgenový pohled ve 3D
- Připojené auto jako skleněný model, který můžete otáčet: motor nebo motory, převodovka, nádrž nebo vysokonapěťová baterie, výfuk, chladič, 12V baterie a zásuvka OBD, rozložené podle karoserie a pohonu rozpoznaného auta.
- Každá řídicí jednotka nalezená při skenování sedí tam, kde obvykle bývá, spojená svými datovými vedeními (CAN pohonu a komfortu); zatímco Caruspex čte, putují malé pakety k dotazované jednotce.
- Závady rozsvítí svou oblast a zbarví odpovědnou řídicí jednotku červeně, živé hodnoty leží na dílu, ke kterému patří, a "Zkontrolovat auto" přesune kameru ke každému nálezu. Vrstvy lze vypínat po jedné.
- Typické rozměry pro karoserii a dobu, ne kopie přesného modelu: konstrukční data a schémata zapojení výrobců nejsou volně dostupná.

![Rentgenový pohled](docs/screenshot-xray.png)

### Živé hodnoty
- Každá hodnota režimu 01 podle SAE J1979 až po PID C4: 145 PID s více než 300 údaji, seskupených a prohledávatelných.
- Stav tachometru, převodový stupeň, teplota oleje, spotřeba paliva, točivý moment, plnicí tlak, teploty výfukových plynů, filtr pevných částic, AdBlue, NOx a hybridní baterie, pokud je auto hlásí.
- Průběžný graf a záznam do CSV.

![Živé hodnoty](docs/screenshot-live.png)

### Měření
- Palubní počítač: spotřeba živě i průměrná, náklady, CO2, vzdálenost a skóre klidného stylu jízdy, i pro auta bez vlastního palubního počítače.
- Výkon: 0 až 100 km/h, 80 až 120, čtvrtmíle a brzdná dráha ze signálu rychlosti (jen na uzavřených silnicích).
- Baterie a alternátor: napětí v klidu, pokles při startu, nabíjecí napětí se zátěží i bez ní, provedení ve čtyřech krocích.

### Kontrola
- Monitory připravenosti srozumitelně, zmrazený rámec, VIN.
- Kontrola součástí v režimu 06 s hodnotou a mezemi, počty vynechání zapalování na válec.
- Názvy jednotek, ID kalibrace a kontrolní součty z režimu 09, pro každou řídicí jednotku.
- Surový terminál ELM327.

### Test adaptéru
- Pravý ELM327, nebo klon? Zeptá se na příkazy každé verze, změří dobu odezvy a poctivě řekne, co z toho plyne.
- Pozná OBDLink a další adaptéry STN.
- Do adaptéru nikdy nic nezapisuje.

![Test adaptéru](docs/screenshot-adapter.png)

### Každý kabel, každá cesta do auta
Caruspex sám zjistí, co je zapojeno a jakým jazykem auto mluví.

| Rozhraní | Příklady | Jak |
|---|---|---|
| ELM327 přes USB, Bluetooth, Bluetooth LE, Wi-Fi | levný ELM327, Vgate, vLinker, Veepeak | sériový port, spárovaný nebo přímo BLE, TCP |
| Čipy STN | OBDLink SX, EX, MX+, CX | jako ELM327, s rychlejším časováním |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A v režimu J2534 | 64bitový ovladač výrobce, CAN s ISO-TP zajišťuje Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | sériový port, nalezený automatickým hledáním portů |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable s candleLight, CAN hat pro Raspberry Pi | can0 a podobné |
| Kabel KKL (VAG 409.1) | kabely s FTDI nebo CP2102 | K-Line, KWP1281 a KWP2000 |
| Kabel ENET | BMW a Mini od řady F | HSFZ přes Ethernet |
| Kabel DoIP | mnoho aut od zhruba 2016 | ISO 13400 přes Ethernet |

Na každém z nich fungují stejné funkce: OBD-II a OBD na UDS, každá řídicí jednotka přes UDS, adresování
BMW, poslech vysílaných hodnot (Tesla), záznam a přehrávání.

![Připojit](docs/screenshot-connect.png)

### Všechny řídicí jednotky přes CAN (UDS)
- S obyčejným ELM327 se Caruspex ptá každé řídicí jednotky postupně: motor, převodovka, ABS, airbag, přístrojový panel, klimatizace a další, u většiny aut od zhruba 2008.
- Paměť závad pro každou jednotku s druhem poruchy (zkrat, přerušení, nevěrohodná hodnota ...), číslem dílu, verzí softwaru a hardwaru.
- Seznamy adres pro VAG, BMW a Mini, Ford, Mazdu, GM, Toyotu, Nissan, Hyundai/Kii, Renault a PSA; pro vše ostatní prohledání obvyklých adres.
- BMW a Mini od řady F s vlastním rozšířeným adresováním BMW přes 6F1 a prohledáním všech 128 diagnostických adres; chybové kódy ve formátu BMW tak, jak je ukazuje ISTA (např. 480A10).
- Mazání po jednotkách, s varováním.

![Všechny řídicí jednotky](docs/screenshot-ecus.png)

### VW a Audi přes kabel KKL (KWP1281 a KWP2000)
- Adaptace škrticí klapky: skutečné základní nastavení v měřicím bloku 060 nebo 098.
- Prohledání všech řídicích jednotek včetně ABS, airbagu, přístrojového panelu a komfortu, s tisknutelnou zprávou.
- Adaptační kanály a testy akčních členů, záměrně jen pro čtení.

### Hodnoty výrobců a vlastní hodnoty
- Baterie elektromobilů: stav zdraví, nabití, napětí článků s rozptylem, teploty, pro Hyundai Kona a Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf a Teslu Model 3 a Y (poslechem na sběrnici CAN); každá hodnota s doloženým zdrojem.
- Hodnoty výrobce pro asi 550 dalších vozidel z projektu [OBDb](https://github.com/OBDb) (baterie, tlak v pneumatikách, olej, tachometr a další), nabízené automaticky pro rozpoznané auto; každá rovnice se kontroluje dekodérem samotného OBDb.
- Načtěte vlastní hodnoty v rozšířeném formátu Torque CSV, s úplným vyhodnocováním rovnic Torque.

![Baterie elektromobilu](docs/screenshot-battery.png)

### Záznam a přehrávání
- Zaznamenejte relaci s autem do souboru a později ji přehrajte bez auta, šikovné při hledání závad.

### Bez auta a adaptéru
- Vestavěné demo auto: jezdí, zahřívá se, má uloženo pár závad a mluví jako skutečný ELM327 přes CAN.
- Offline lexikon všech kódů, filtrovatelný podle značky.

### Vzhled a ovládání
- Nové rozhraní ve stylu Odysync: klidné, tmavé, s plynulými přechody.
- Připojení krok za krokem s automatickým hledáním na všech portech a při nezdaru krátký kontrolní seznam pro daný druh připojení (ovladač, párování, zapalování, přenosová rychlost, klony adaptérů) s tím, co už Caruspex zjistil.
- Režim pro začátečníky a profesionály: začátečníci vidí stránky pro každý den, dílenské nástroje (KKL, adaptace, surové příkazy, adresy) se objeví v profesionálním režimu.
- 18 jazyků; nastavení se projeví hned.
- Řekne vám, že je venku nová verze: při startu se zeptá GitHubu na nejnovější vydání a na nic jiného (žádná data
  o vás ani o autu); v nastavení lze vypnout.

## Podporovaná vozidla

Každé vozidlo s OBD-II nebo EOBD: v Evropě benzinová auta od roku 2001 a diesely od 2004, v USA od 1996. Starší VW, Audi, Seat a Škoda od zhruba 1990 fungují přes kabel KKL. Stránka Vozidla řekne, která cesta funguje pro dané auto.

| Období | Sběrnice | Příklady |
|---|---|---|
| do zhruba 1995 | specifická pro výrobce | VAG s KKL přes konektory 2x2, jinak jen dílenské nástroje |
| 1996 do zhruba 2003 | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| od zhruba 2004 | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, moderní japonská auta |
| USA do zhruba 2005 | SAE J1850 PWM a VPW | Ford, Chevrolet |
| nejnovější auta | OBD na UDS (SAE J1979-2) | v USA povinné od modelového roku 2027, už v některých novějších autech; rozpoznáno a přeloženo automaticky |
| BMW a Mini od řady F | ENET (HSFZ přes Ethernet) nebo D-CAN s adresováním 6F1 | každá řídicí jednotka |
| mnoho aut od zhruba 2016 | DoIP (ISO 13400) s kabelem DoIP | OBD přes Ethernet, nalezeno automaticky |
| Tesla Model 3 a Y | bez OBD: vysílané rámce CAN | baterie, články, teploty, rychlost, stav tachometru poslechem (kabel adaptéru k diagnostickému konektoru) |

Čistě elektrická auta nemusejí nabízet emisní data OBD-II. Některá přesto odpovídají částečně; u aut bez jakékoli odpovědi, ale s provozem na sběrnici, se Caruspex připojí pro poslech a přečte, co znají vestavěné balíčky (zatím Tesla Model 3 a Y).

## Podporované adaptéry

- Jakýkoli adaptér kompatibilní s **ELM327**: USB, klasický Bluetooth (přímo přes RFCOMM ve Windows a Linuxu po spárování, nebo přes jeho port COM), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE a další adaptéry s obvyklými službami GATT; nalezené
  skenem, bez párování) nebo Wi-Fi přes TCP
- **Kabel ENET** (OBD na RJ45) pro BMW a Mini od řady F: Caruspex najde auto v síti
  (HSFZ, porty 6811 a 6801) a přečte každou řídicí jednotku přes Ethernet, bez čipu adaptéru mezi tím. Počítač
  potřebuje automatickou adresu (169.254.x.x), kterou Windows, macOS i Linux přidělí samy, když neodpoví žádný DHCP.
  I zde Caruspex jen čte; kódování a flashování zůstává na E-Sys, ISTA nebo tuningových nástrojích.
- Jakýkoli **kabel KKL (VAG 409.1)**. Kabely založené na FTDI nebo CP2102 jsou na K-Line mnohem spolehlivější než
  nejlevnější klony; pro KWP1281 se doporučuje originální FTDI FT232RL.

Proprietární rozhraní jako Delphi DS150E, Autocom nebo VAS 5054A nejsou podporována, nemluví ELM327.

| Funkce | ELM327 | KKL 409.1 |
|---|---|---|
| Čtení a mazání závad motoru | ano | ano |
| Živé hodnoty, zmrazený rámec, připravenost | ano | ano |
| ABS, airbag, přístrojový panel, komfort | **ne** | ano |
| Měřicí bloky výrobce | **ne** | ano |
| **Adaptace škrticí klapky** | **ne** | ano |

Je to hardwarové omezení, ne softwarové: KWP1281 je blokový protokol, v němž každý bajt potvrzuje
druhá strana. Firmware ELM327 zpracuje jen celé dvojice požadavek/odpověď.
Kabel KKL propouští K-Line surově, takže protokolem může mluvit Caruspex sám.

### Dva protokoly VAG, rozpoznávané automaticky

| Protokol | Typický pro | Základní nastavení |
|---|---|---|
| KWP1281 (blokový protokol) | motory 1.4 MPI | vlastní příkaz (blok 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI s Bosch MED | vlastní diagnostická relace, pak čtení bloku |

Adaptační kanály a testy akčních členů jsou zatím implementovány na cestě KWP1281; u KWP2000
to aplikace oznámí, místo aby mlčky selhala.

Zatím neimplementováno: diagnostika výrobce přes CAN s VW TP 2.0 (vozidla od zhruba 2004).

## Technologie

| Oblast | Technologie |
|---|---|
| Jazyk | Rust (edice 2024) |
| Rozhraní | Tauri 2 s React 19 a TypeScript, sestavováno přes Vite |
| Knihovny | `serialport` (sériové porty), `btleplug` (Bluetooth LE), `libloading` (ovladače J2534) |
| Lokalizace | Jedna sada souborů JSON v `translations/` pro jádro i rozhraní, 18 jazyků |
| Sestavení | Workspace Cargo, npm pro rozhraní |
| Testy | `cargo test` proti simulovaným jednotkám, falešným bránám a falešnému ovladači J2534 |
| Nástroje | Python 3 (generátory kódu a tabulek, kontroly překladů, simulátor ELM327) |
| Platformy | Windows 10 a 11, Linux, macOS |

## Stažení

Hotové balíčky jsou na [stránce vydání](https://github.com/SenseiIssei/Caruspex/releases):

| Systém | Soubor | Poznámky |
|---|---|---|
| Windows 10 a 11, 64bitový | `Caruspex-<version>-windows-x64-setup.exe` | Instaluje se bez práv správce a umí přidat `caruspex_cli` do cesty pro vyhledávání. Nainstalovanou verzi 0.8 aktualizuje na místě a běhové prostředí WebView2 nainstaluje, jen pokud chybí |
| Windows 10 a 11, 64bitový | `Caruspex-<version>-windows-x64.zip` | Přenosná verze, rozbalit a spustit. Vyžaduje běhové prostředí WebView2, které Windows 10 a 11 už mají |
| macOS 13 a novější, Apple Silicon | `Caruspex-<version>-macos.dmg` | Přetáhněte aplikaci do Aplikací |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` a spustit |
| Debian a Ubuntu, 64bitový | `Caruspex-<version>-linux-amd64.deb` | Instalace přes `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Jen nástroj příkazového řádku | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Pro skripty a servery, bez rozhraní |

Balíčky zatím nejsou podepsané. Windows SmartScreen se při prvním spuštění zeptá ("Další informace", "Přesto spustit"); v macOS
otevřete aplikaci poprvé pravým kliknutím a "Otevřít".

## Sestavení

Požadavky: stabilní toolchain Rustu (edice 2024, Rust 1.85 nebo novější), Node 24 s npm a Python 3 pro
generátory a kontroly překladů. V Linuxu systémové balíčky, které potřebuje rozhraní a spojení s adaptéry
(názvy pro Debian a Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows potřebuje běhové prostředí WebView2 (součást Windows 10 a 11), macOS nepotřebuje nic navíc.

Závislosti rozhraní nainstalujte jednou, pak sestavte nástroj příkazového řádku a aplikaci:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` spustí aplikaci s živým přenačítáním, když na ní pracujete. Samotné
rozhraní, bez okna Tauri, běží v běžném prohlížeči s vestavěným demo autem:

```bash
npm --prefix app run dev
```

Otevřete `http://localhost:5287/?demo` a hned se připojíte k demo autu. `?lang=de` zvolí jazyk a
`?theme=light` světlý motiv, například `?demo&lang=de&theme=light`. Bez shellu Tauri není
přístup k adaptérům, takže verze v prohlížeči zná jen demo auto.

### Oprávnění k sériovému portu v Linuxu

Pro přístup k adaptéru bez `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Pak se odhlaste a znovu přihlaste.

## Výstupy sestavení

| Výstup | Popis |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | Aplikace s rozhraním Tauri (`tauri build` zapíše také instalátory a balíčky do `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Skriptovatelný nástroj příkazového řádku nad stejným jádrem, viz [Nástroj příkazového řádku](#nástroj-příkazového-řádku) |

### Balíčky a vydání

`bash tools/package.sh` sestaví balíčky pro aktuální systém do `dist/`: instalátor a dva archivy zip ve
Windows (instalátor potřebuje [Inno Setup 6](https://jrsoftware.org/isinfo.php)), dmg a archiv nástroje příkazového řádku
v macOS, AppImage, `.deb` a archiv nástroje příkazového řádku v Linuxu. `bash tools/smoke_test.sh` je pak nainstaluje nebo
otevře jako uživatel, spustí oba programy a zase je odstraní. CI dělá obojí při každém běhu.

Vydání vzniká ze značky. `bash tools/release.sh <version>` ho připraví na vlastní větvi, aniž by cokoli odeslal:
nastaví verzi v `Cargo.toml` a `app/package.json`, vytvoří `changelog/<version>.md` (německy a anglicky),
spustí kontroly, testy, balení a kouřový test a obnoví snímky obrazovky v README (`tools/screenshots.sh de` a
`tools/screenshots.sh en`). Po sloučení pull requestu vytvořte značku `v<version>` a odešlete ji. CI sestaví všechny
tři systémy a vydání publikuje s balíčky a seznamem změn.

CI (`.github/workflows/build.yml`) nejdřív spustí formátování, kontrolu generovaných tabulek, kontrolu překladů, sestavení
rozhraní a clippy v Linuxu, pak testuje, balí a kouřově testuje Windows, Linux a macOS.
Druhý workflow (`.github/workflows/data-check.yml`) jednou měsíčně hledá nové kódy VIN, vozidla a soubory DBC
v upstreamu a otevře issue s tím, co je nového.

## Testy

```bash
cargo test --workspace
```

Asi 370 testů proběhne za pár sekund, všechny bez auta. Pokrývají parser odpovědí ELM327, dekódování chybových kódů,
převod PID, profily vozidel a dekodér VIN, služby OBD-II a UDS proti demo autu,
úplné relace KWP1281 a KWP2000 proti simulovaným řídicím jednotkám, ENET (HSFZ) a DoIP proti falešným bránám,
SLCAN a cestu J2534 proti falešnému ovladači, záznamy a přehrávání a analýzu (kontrola zdraví, test baterie,
palubní počítač, časovač výkonu). Testy nástroje příkazového řádku spouštějí sestavený `caruspex_cli` a porovnávají jeho výstup se
vzorovými soubory v `crates/caruspex-cli/tests/golden/`, které obsahují to, co dřívější verze v C++ vypsala pro stejné
příkazy, takže každý rozdíl oproti starému nástroji vyjde najevo.

Kontroly, které CI spouští kromě testů:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

První dvě jsou formátování a lintování. `tools/i18n_rust.py --strict` selže, když text v jádru chybí v
souborech překladů nebo když chybí překlad v některém z 18 jazyků; sestavení rozhraní kontroluje své vlastní
texty stejně, když je nastaveno `I18N_STRICT=1`. Spusťte `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` a
`gen_pids.py` pro obnovení generovaných tabulek; CI selže, pokud se liší od toho, co je v repozitáři.

## Vyzkoušení bez auta

Nejrychlejší je vestavěné demo auto: vyberte "Demo auto" na stránce Připojit, nebo spusťte s

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` vypíše všechny volby aplikace: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` a `--size`.

Simulátor v Pythonu hraje buď motor Audi A2 na K-Line, nebo s `--can` auto s CAN s motorem
a převodovkou (hlavičky, ISO-TP, režimy 06 a 09 pro každou řídicí jednotku):

```bash
python tools/mock_elm327.py --can
```

Pak připojte aplikaci (ve Windows je program `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

Nástroj příkazového řádku funguje se simulátorem také:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Nástroj příkazového řádku

`caruspex_cli <command>` s `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` nebo `--replay FILE`:

| Příkaz | Co dělá |
|---|---|
| `read` | Kontrolka, připravenost a chybové kódy, s významem pro danou značku, pokud ji VIN prozradí |
| `vin` | VIN, co říká (výrobce, země, modelový rok, kontrolní číslice) a odpovídající auto z katalogu |
| `freeze` | Každý zmrazený rámec, jeden na každou uloženou závadu |
| `health` | Kontrola zdraví: závady, připravenost, režim 06 a živá řada měření (`--seconds N`, výchozí 15) |
| `probe` | Test adaptéru: čím ELM327 doopravdy je, jaké příkazy zná, doby odezvy |
| `uds` | UDS na CAN: identifikace a paměť závad každé řídicí jednotky (`--ids A,B,...` pro vlastní adresy) |
| `custom` | Hodnoty výrobce a vlastní hodnoty ze vestavěného balíčku (`--pack NAME`) nebo z Torque CSV (`--csv FILE`), jednorázově |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Živé hodnoty, VIN a první zmrazený rámec, mazání, vše po řídicích jednotkách, sériové porty, autotest kabelu KKL |
| `enet`, `interfaces` | Hledá v síti auta BMW a Mini a auta s DoIP; vypíše rozhraní CAN tohoto počítače (ovladače J2534, SocketCAN) |
| `version`, `help` | Verze programu, úplný seznam příkazů a voleb |

`--json` vypíše `read`, `vin`, `health`, `probe`, `freeze`, `uds` a `custom` jako JSON pro skripty, `--record FILE`
uloží celou relaci s adaptérem pro `--replay`, `--fast-timing` přepne adaptér na agresivní adaptivní
časování, `-v` zaznamená každý příkaz adaptéru (spolu s `--json` na stderr). `caruspex_cli help` vypíše vše.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Snímky obrazovky na této stránce pocházejí z demo auta, pořídila je sama aplikace:
`--demo --page faults --shot shot.png --size 1440x920` otevře stránku, po několika sekundách uloží obrázek okna
a skončí (`tools/screenshots.sh` pořídí celou sadu).

## Struktura projektu

```
crates/caruspex-i18n/       Jazyky: němčina a angličtina v kódu, ostatních 16 z translations/
crates/caruspex-vehicles/   Dekodér VIN a katalog vozidel (značky, modely, generace, konektory, sběrnice)
crates/caruspex-data/       Chybové kódy, PID, adresy řídicích jednotek, přiřazení kódu ke komponentě
crates/caruspex-link/       Cesty k adaptéru (sériová, TCP, Bluetooth LE), ovladač ELM327, test adaptéru
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 a KWP1281 (K-Line, bajt po bajtu)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Záznamy a přehrávání, vlastní hodnoty (Torque CSV, balíčky PID), zapisovač JSON
crates/caruspex-analysis/   Kontrola zdraví, průvodce jízdním cyklem, palubní počítač, časovač výkonu, test baterie
crates/caruspex-demo/       Vestavěné demo auto: ELM327 se simulovanými řídicími jednotkami
crates/caruspex-cli/        Nástroj příkazového řádku, s testy podle vzorových souborů
app/src/                    Rozhraní: React 19 a TypeScript (stránky, shell, stav, zpráva, i18n)
app/src-tauri/              Aplikace Tauri: okno, příkazy, kontrola aktualizací, volby příkazového řádku
translations/               Společné soubory překladů pro jádro a rozhraní, 18 jazyků
data/                       Katalog vozidel jako ručně udržované CSV, zdroje řídicích jednotek a PID
tools/                      Generátory, kontroly překladů, skripty balení, vydání a snímků obrazovky, simulátor ELM327
docs/                       Snímky obrazovky pro README
packaging/                  Skript instalátoru Windows, soubor pracovní plochy Linuxu, Info.plist pro macOS
changelog/                  Poznámky k vydání pro každou verzi, německy a anglicky
```

Databáze chybových kódů je generovaná: `tools/gen_dtc.py` rozvíjí pravidelné rodiny kódů EOBD a
vybrané seznamy v `tools/dtc/` do `crates/caruspex-data/src/generated/*.rs`. Po změně dat ho spusťte znovu:

```bash
python tools/gen_dtc.py
```

Katalog vozidel funguje stejně: upravte soubory CSV v `data/vehicles/` a pak spusťte

```bash
python tools/gen_vehicles.py
```

Adresy řídicích jednotek a balíčky PID mají `tools/gen_ecus.py` a `tools/gen_pids.py`. Překlady jsou popsány
v [translations/TRANSLATING.md](translations/TRANSLATING.md); po přidání textu do kódu
`python tools/i18n_rust.py` vypíše, co je ještě potřeba přeložit.

## Poznámka k mazání kódů

Režim OBD 04 maže paměť závad, zmrazený rámec a monitory připravenosti. Hned poté
auto neprojde emisním testem, dokud neabsolvuje několik úplných jízdních cyklů, a kód, jehož příčina
stále trvá, se po krátké jízdě vrátí. Mazání není oprava.

## Upozornění

Caruspex je hobby projekt a poskytuje se bez záruky. Používáte ho na vlastní riziko, jen na vozidlech,
na kterých smíte pracovat, a nikdy za jízdy. Všechny názvy produktů a značek jsou ochranné známky jejich
příslušných vlastníků; Caruspex není spojen s žádným výrobcem vozidel ani jím podporován.

## Licence

MIT, viz [LICENSE](LICENSE). Hodnoty výrobce pro stovky modelů jsou převedeny z [OBDb](https://github.com/OBDb) a stejně jako OBDb podléhají licenci CC BY-SA 4.0: leží odděleně od kódu v [data/obdb](data/obdb) s licencí a seznamem zdrojů ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)). Přibalená písma podléhají licenci SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) a
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
