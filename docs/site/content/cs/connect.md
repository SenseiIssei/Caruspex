# Průvodce připojením a zásuvka OBD

## Kde je zásuvka?

Každé auto prodané v Evropě od roku 2001 (benzín) nebo 2004 (diesel) a v USA od roku 1996 má 16pinovou zásuvku OBD na dosah řidiče. Obvyklá místa:

- pod palubní deskou, vlevo nebo vpravo od sloupku řízení,
- nad pedály,
- v bočním obložení prostoru pro nohy,
- za krytem pojistkové skříňky,
- ve středové konzoli, někdy za malou klapkou.

U více než 200 generací aut a 35 motorek zná Caruspex přesné místo. Průvodce připojením ho vyznačí v půdorysu a v detailu strany řidiče; u motorek ukáže boční pohled s diagnostickou zásuvkou a potřebným kabelem adaptéru.

> Seznam umístění zásuvky je naše vlastní sbírka a nebyl ověřen u každého auta. Pokud místo u vašeho auta nesedí, dejte nám prosím vědět [zprávou o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Průvodce připojením

Průvodce se otevře sám při prvním spuštění. Zeptá se na čtyři věci a každý krok lze přeskočit:

1. **Jaký adaptér?** ELM327 přes USB, Bluetooth nebo Wi-Fi, kabel ENET nebo kabel KKL. Obrázky pomohou je rozlišit.
2. **Jaké auto?** Vyberte ho z katalogu a průvodce ukáže, kde je zásuvka.
3. **Zapalování zapnuté.** Většina řídicích jednotek odpovídá jen při zapnutém zapalování.
4. **Připojit.** Caruspex prohledá porty a zkouší protokoly.

Později najdete průvodce v nastavení a na stránce Připojit.

## Hledání a zapamatování

Při automatickém hledání projde Caruspex všechny sériové porty, na každém se zeptá na ELM327 a pak jeden po druhém zkouší protokoly auta. Jakmile se auto jednou připojí, Caruspex si k jeho VIN zapamatuje cestu (adaptér, port, protokol, přenosovou rychlost) a příště ji zkusí jako první, takže připojení je rychlejší.

## Když se nepřipojí

Když žádný adaptér neodpovídá nebo auto mlčí, ukáže Caruspex krátký kontrolní seznam pro daný druh připojení spolu s tím, co už zjistil: ovladač nainstalovaný, Bluetooth spárované, zapalování zapnuté, správná přenosová rychlost, klon adaptéru, který jen předstírá novější verzi. Stránky v části [adaptéry a rozhraní](interfaces.md) podrobně vysvětlují každý druh.
