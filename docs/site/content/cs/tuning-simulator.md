# Simulátor tuningu

Stránka **Simulátor tuningu** (profesionální režim, v části Měřit) je virtuální BMW N55: řadový šestiválec 3,0 litru s jedním turbem twin-scroll z F30 335i a F32/F36 435i, 306 k a 400 Nm. Měňte plnicí tlak, zapalování a lambdu po pásmech otáček a uvidíte, co na to říká model, dlouho předtím, než se čehokoli v autě dotknete.

> **Pouze simulace.** Caruspex nikdy nezapisuje mapy motoru a nikdy nepřeprogramovává. Nezná žádný formát souborů pro flashování, žádný kontrolní součet ani adresu mapy a nekopíruje žádný komerční tuning. Skutečný tuning patří k zavedeným nástrojům a do dílny a změna potřebuje schválení (v Německu u TÜV). Bez něj auto přijde o schválení technické způsobilosti.

![Simulátor tuningu](screenshot-tune.png)

## Co můžete měnit

- Odchylky v pěti pásmech otáček (1000 až 2000, 2000 až 3500, 3500 až 5000, 5000 až 6000, 6000 až 7000): plnicí tlak v barech, zapalování ve stupních, lambda.
- Palivo (OČ 95, 98, 100 nebo 102), venkovní teplotu, tlak vzduchu a převodovku (automat 8HP45 nebo šestistupňová manuální).
- **Sériový** vrátí odchylky na nulu, **Obnovit** i podmínky.

Stránka ukazuje točivý moment a výkon sériově proti změněnému, každý limit, kterého se změna dotkne, s důvodem, a vnitřní hodnoty: plnicí tlak, plnicí vzduch, zapalování, rezervu klepání, teplotu výfuku, turbo, vysokotlaké čerpadlo, střední efektivní tlak.

## Jak model počítá

1. **Vzduch.** Cílový plnicí tlak omezuje to, co turbo při těchto otáčkách zvládne, jeho tlakový poměr a jeho průtok. Kompresor vzduch ohřeje, mezichladič část tepla zase odebere. Tlak, teplota a plnicí účinnost dávají hmotnost vzduchu.
2. **Palivo a zapalování.** Palivo vyplývá ze vzduchu a lambdy. Zapalování je sériový předstih plus vaše odchylka, ale nikdy dřív, než dovolí mez klepání: regulace klepání motoru by ho vzala zpět. Mez se posouvá s oktanovým číslem, teplotou vzduchu, plnicím tlakem a lambdou.
3. **Výkon.** Energie paliva krát účinnost, která klesá se vzdáleností od nejlepšího předstihu, minus tření dává točivý moment a výkon.
4. **Limity.** Turbo, teplota výfuku, vysokotlaké čerpadlo, vstřikovače, předzápal při nízkých otáčkách, převodovka, plnicí vzduch a směs dávají každý varování jednoduchými slovy.

Celá křivka (25 bodů od 1000 do 7000 ot/min) trvá ve vydané verzi asi 0,015 ms.

## Publikované údaje

| Hodnota | Číslo | Zdroj |
|---|---|---|
| Zdvihový objem, vrtání, zdvih, komprese | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Točivý moment | 400 Nm od 1200 do 5000 ot/min | Wikipedia "BMW N55", údaje BMW pro 435i |
| Výkon | 225 kW (306 k) při 5700 až 6000 ot/min | Wikipedia (5700 až 5800), BMW (5800 až 6000) |
| Maximální otáčky | 7000 ot/min | Wikipedia "BMW N55" |
| Jmenovitý vstupní moment 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Předpoklady

Vnitřní mapy N55 nikdo nezveřejňuje pod svobodnou licencí. Tyto hodnoty jsou mé předpoklady; každý je zdůvodněn v [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- **Sériový plnicí tlak** je modelem zpětně dopočítaný z publikované křivky (asi 0,7 až 0,8 bar do 5000 ot/min, výš méně). Že sériová křivka sedí, proto ukazuje soudržnost, ne pravdu.
- Plnicí účinnost 0,86 až 0,97, indikovaná účinnost 0,39, tření kvadraticky podle otáček.
- Zapalování omezené klepáním 3 až 15 stupňů před horní úvratí s rezervou 1,5 stupně; posouvá se o 1 stupeň na oktanové číslo, 0,1 stupně na kelvin vzduchu, 6 stupňů na bar a 2,5 stupně na 0,1 lambdy.
- Turbo: tlakový poměr do 2,55, korigovaný průtok do 0,32 kg/s, účinnost 0,72. Účinnost mezichladiče 0,85, při vysokém průtoku méně.
- Mez teploty výfuku 980 °C, vysokotlaké čerpadlo 0,027 kg/s, okno vstřiku 40 % cyklu, riziko předzápalu nad 18 bar středního efektivního tlaku pod 2500 ot/min, manuální převodovka asi 450 Nm.
- Palivo E5: poměr vzduchu a paliva 14,4, 42,4 MJ/kg.

## Jak je to přesné?

- Sériově proti publikované křivce: 400 Nm s přesností 1 % od 1250 do 5000 ot/min, 306 k při 6000 ot/min a 304 k při 5750 ot/min. Testy povolují 3 %.
- Model zatím nebyl ověřen na skutečném N55. Každé číslo je odhad: asi ±8 % u momentu a výkonu, ±0,1 bar plnicího tlaku, ±10 K plnicího vzduchu, ±3 stupně rezervy klepání, ±60 K teploty výfuku.

## Porovnání s vaším autem

1. Připojte se, otevřete **Živé hodnoty** s otáčkami, zatížením, tlakem v sání, teplotou nasávaného vzduchu, předstihem, lambdou a hmotností vzduchu a spusťte záznam.
2. Jednou zrychlete na plný plyn celým rozsahem otáček, jen tam, kde je to dovolené a bezpečné.
3. Otevřete záznam na stránce simulátoru. Caruspex si vezme body při plném zatížení, seskupí je do pásem po 500 ot/min a ukáže naměřené proti simulovanému: tlak v sání a plnicí vzduch s vašimi mapami, hmotnost vzduchu a zapalování s naměřeným tlakem a teplotou.

OBD hlásí tlak v sání jen do 255 kPa a který snímač řídicí jednotka používá pro teplotu sání, není zdokumentováno. **Zobrazit příklad** naplní tabulku vymyšlenými hodnotami, abyste viděli, jak se čte.

## Dobré vědět

- Víc točivého momentu než sériově při nízkých otáčkách zvyšuje riziko předzápalu, který může naráz zničit písty.
- Zapalování dřívější než mez klepání nic nepřinese: regulace klepání ho vezme zpět.
- Lepší palivo samo přidá rezervu, ne výkon: sériová mapa zapalování je strop.
