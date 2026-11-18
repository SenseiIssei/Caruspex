# ELM327 přes USB

Koncovka do zásuvky OBD s kabelem do portu USB. Je to nejspolehlivější druh ELM327: žádné párování, žádné rádio, žádné vybité baterie.

## Ovladač

Uvnitř koncovky dělá čip USB z kabelu sériový port (port COM ve Windows). Jaký ovladač je potřeba, závisí na tomto čipu:

| Čip | Ovladač |
|---|---|
| FTDI FT232 | obvykle ho nainstaluje Windows sám, v Linuxu a macOS je zabudovaný |
| CH340 / CH341 | obvykle ho nainstaluje Windows sám, v Linuxu a macOS je zabudovaný |
| Silicon Labs CP210x | z webu výrobce, pokud ho Windows nenajde |
| Prolific PL2303 | z webu výrobce; staré klony často fungují jen se starým ovladačem |

Po zapojení ukáže Správce zařízení ve Windows adaptér v části "Porty (COM a LPT)".

## Připojení

1. Zapojte adaptér do auta, zapněte zapalování, zapojte kabel do počítače.
2. Na stránce **Připojit** vyberte **USB nebo port COM**.
3. Nechte port na **Hledat automaticky** nebo ho vyberte ze seznamu a připojte se.

Automatické hledání zkouší každý sériový port na obvyklých rychlostech. Pokud rychlost svého adaptéru znáte, můžete ji nastavit v nastavení v položce **Přenosová rychlost adaptéru**.

## Linux a macOS

V Linuxu se adaptér objeví jako `/dev/ttyUSB0` nebo `/dev/ttyACM0`. Váš uživatel musí být ve skupině `dialout` (viz [Instalace](install.md)). V macOS se objeví jako `/dev/cu.usbserial-...` nebo `/dev/cu.wchusbserial...`.

## Typické problémy

- **Neobjeví se žádný port**: chybí ovladač, nebo je kabel jen nabíjecí.
- **Adaptér odpovídá, auto ne**: vypnuté zapalování, nebo je adaptér klon, který protokol auta nezvládne. Víc vám řekne [test adaptéru](interfaces.md).
- **Velmi pomalé živé hodnoty u starších aut**: pomalá je samotná K-Line, není to chyba adaptéru.
