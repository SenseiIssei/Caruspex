# DoIP (diagnostika přes IP)

Mnoho aut zhruba od roku 2016 lze diagnostikovat přes Ethernet podle ISO 13400, čemuž se říká DoIP. Kabel DoIP propojí zásuvku OBD se síťovým portem počítače.

## Připojení

1. Zapojte kabel, zapněte zapalování a počkejte, až počítač dostane automatickou adresu (169.254.x.x), což může trvat až minutu.
2. Na stránce **Připojit** vyberte **Síťový kabel** a pole nechte prázdné. Caruspex v síti vyhledá vozidla DoIP a sám pozná, jestli auto mluví DoIP, nebo HSFZ od BMW.

## Co funguje

OBD přes DoIP a řídicí jednotky přes UDS, jejichž adresy Caruspex pro danou značku zná. Seznamy adres pro auta s DoIP jsou zatím krátké; záznamy ze skutečných aut je doplňují (co sdílený záznam obsahuje, viz [soukromí](privacy.md)).

## Jaký kabel?

DoIP potřebuje ethernetové vodiče na pinech 3, 11, 12 a 13 zásuvky OBD a aktivační vodič na pinu 8. Kabely prodávané pro konkrétní značku bývají zapojené přesně tak; kabel BMW ENET je jedním z nich.

## Příkazový řádek

```
caruspex_cli enet
```

vypíše vozidla BMW a DoIP nalezená v síti.
