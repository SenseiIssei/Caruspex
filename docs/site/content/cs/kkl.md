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

## Typické problémy

- **Řídicí jednotka neodpovídá**: je zapnuté zapalování? Probuzení rychlostí 5 baudů potřebuje přesné časování; rozbočovače USB a některé virtuální stroje ho kazí, zapojte kabel přímo.
- **Po chvíli spojení spadne**: levné kabely ztrácejí bajty. Než začnete podezírat auto, zkuste jiný kabel.
