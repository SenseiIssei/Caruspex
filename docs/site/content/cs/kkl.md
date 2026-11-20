# Kabel KKL (VAG 409.1)

Kabel KKL je kabel USB se sériovým čipem a budičem K-Line, bez ELM327 uvnitř. Caruspex řídí K-Line sám, bajt po bajtu. Přesně to potřebují starší VW, Audi, Seat a Škoda: jejich protokol KWP1281 potvrzuje každý jednotlivý bajt, a to žádný ELM327 neumí.

## Co funguje

- Každá řídicí jednotka VW, Audi, Seat a Škoda zhruba z let 1990 až 2004: motor, převodovka, ABS, airbag, přístrojový panel, komfort a další.
- KWP1281 a KWP2000 (ISO 14230), automaticky rozpoznané podle klíčových bajtů po probuzení.
- Paměť závad každé řídicí jednotky, bloky naměřených hodnot a skutečné základní nastavení škrticí klapky (blok naměřených hodnot 060 nebo 098).
- Adaptační kanály jsou záměrně jen pro čtení. Test akčních členů spíná skutečné díly a spustí se až po vašem potvrzení.

Tyto stránky jsou v **profesionálním režimu**: **Řídicí jednotky přes KKL**, **Škrticí klapka** a **Adaptace a testy akčních členů**. Každá si kabel otevře sama: vyberte sériový port, řídicí jednotku, spustit.

## Jaký kabel?

Kabely s pravým FTDI FT232RL nebo CP2102 jsou na K-Line mnohem spolehlivější než nejlevnější klony. Pro KWP1281 se doporučuje pravý FTDI. Kabel musí K-Line propouštět beze změny: rozhraní "HEX-USB" s vlastním firmwarem nefungují.

Auta zhruba před rokem 1996 často nemají zásuvku OBD, ale konektor 2x2 v motorovém prostoru nebo pod palubní deskou. Redukce z 2x2 na 16pinovou zásuvku je levná.

## Kontrola kabelu

Nástroj pro příkazový řádek otestuje kabel bez auta:

```
caruspex_cli kklcheck --port COM3
```

## První test na autě

Stránka **První test na autě** v profesionálním režimu vás provede první relací s novým kabelem KKL. Je připravená pro Audi A2 a funguje na každém VW, Audi, Seatu nebo Škodě s K-Line. Pouze čte: nic se nemaže, žádný test akčních členů, žádné základní nastavení. Port se otevře až při spuštění testu.

1. **Zkontrolujte kabel bez auta.** Zapojte kabel jen do USB, vyberte jeho port a stiskněte **Zkontrolovat kabel**. Caruspex přečte čip (FTDI, CH340, Prolific, CP210x), u FTDI časovač latence, a změří bity adresy 5 baud na lince break. Bez auta se echo nevrací, tak to má být.
2. **Nastavte časovač latence na 1 ms** (kabely FTDI ve Windows): Správce zařízení, Porty (COM a LPT), dvakrát klikněte na USB Serial Port, Nastavení portu, Upřesnit, Latency Timer (msec) na 1, OK, pak kabel jednou odpojte a znovu zapojte. S výchozími 16 ms může odpověď na bajty klíče nestihnout své okno 50 ms. Caruspex toto nastavení čte, ale nikdy ho nemění.
3. **Na autě:** zapalování zapnuté, motor vypnutý, kabel pevně v zásuvce OBD a přímo v počítači (žádný rozbočovač USB), žádný jiný diagnostický program otevřený. Zaškrtněte seznam a stiskněte **Spustit první test**.
4. Test probudí motor (adresa 01) a změří každý bit adresy, synchronizační bajt 0x55, bajty klíče a odpověď na ně. Pak přečte identifikaci, paměť závad a měřicí bloky 001 až 003, potom přístrojový panel 17, ABS 03, airbag 15, komfort 46 a centrální elektriku 09. Řídicí jednotka, která neodpovídá, možná prostě není namontovaná.
5. **Uložit zprávu** uloží textový soubor a soubor JSON do složky `test-reports` ve složce nastavení Caruspexu. **Otevřít zprávu o testu na GitHubu** za vás vyplní formulář zprávy. VIN a číslo imobilizéru jsou zkrácené, dokud políčko nezaškrtnete.

Co výsledky znamenají:

| Výsledek | Co dělat |
|---|---|
| Žádné echo | Kabel neslyší sám sebe: konektor nesedí, nebo na pinu 16 chybí 12 V (zkontrolujte pojistku diagnostické zásuvky). Bez auta je to normální. |
| Echo je zkreslené | Jiné zařízení v zásuvce, přepínač kabelu K+DCAN není na K-Line, nebo vadný kabel. |
| Budicí adresa je taktovaná příliš nerovnoměrně | Bity se od 200 ms odchylují o víc než 20 ms: žádný rozbočovač USB, zavřete ostatní programy, časovač latence 1 ms. Caruspex to zkusí ještě jednou s dřívějším voláním break. |
| Kabel v pořádku, ale auto neodpovídá | Je zapnuté zapalování? Adresa 01 je motor. Auta zhruba od roku 2005 se diagnostikují přes CAN. |
| Odpověď na bajty klíče přišla pozdě | Nastavte časovač latence na 1 ms. |

## Typické problémy

- **Řídicí jednotka neodpovídá**: je zapnuté zapalování? Probuzení rychlostí 5 baudů potřebuje přesné časování; rozbočovače USB a některé virtuální stroje ho kazí, zapojte kabel přímo.
- **Po chvíli spojení spadne**: levné kabely ztrácejí bajty. Než začnete podezírat auto, zkuste jiný kabel.
