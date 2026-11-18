# Kabel BMW ENET

BMW a Mini od řady F (zhruba od roku 2008) mají v zásuvce OBD Ethernet. Kabel ENET vede ze zásuvky OBD do síťového portu počítače, bez čipu adaptéru mezi tím. Caruspex mluví s gatewayem auta protokolem HSFZ (porty 6811 a 6801) a dostane se ke každé řídicí jednotce.

## Připojení

1. Zapojte kabel do auta a do síťového portu počítače (nebo do síťového adaptéru USB). Zapněte zapalování.
2. Počkejte až minutu: bez serveru DHCP si Windows, macOS i Linux přidělí počítači automatickou adresu (169.254.x.x) samy.
3. Na stránce **Připojit** vyberte **Síťový kabel** a pole nechte prázdné: Caruspex auto v síti vyhledá. Pokud hledání nic nenajde, zadejte adresu auta.

## Co funguje

- Každá řídicí jednotka přes UDS s adresováním BMW, chybové kódy ve formátu BMW tak, jak je ukazuje ISTA (například 480A10).
- Paměť závad, identifikace a mazání u každé řídicí jednotky, živé hodnoty přes OBD.
- I tady Caruspex jen čte. Kódování a flashování zůstává dílenským programům.

## Jaký kabel?

Obyčejný kabel ENET má mezi piny 8 a 16 rezistor, který v autě zapne Ethernet. Kabely bez něj zůstanou mrtvé. Síťové porty 1 Gbit/s fungují, auto mluví rychlostí 100 Mbit/s.

## Příkazový řádek

```
caruspex_cli ecus --enet auto
```
