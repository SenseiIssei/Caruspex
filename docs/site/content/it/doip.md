# DoIP (Diagnostics over IP)

Molte auto dal 2016 circa si possono diagnosticare via Ethernet secondo la ISO 13400, detta DoIP. Un cavo DoIP collega la presa OBD alla porta di rete del computer.

## Collegamento

1. Collega il cavo, accendi il quadro e aspetta che il computer abbia il suo indirizzo automatico (169.254.x.x), il che può richiedere fino a un minuto.
2. Nella pagina **Collega** scegli **Cavo di rete** e lascia vuoto il campo. Caruspex cerca veicoli DoIP in rete e riconosce da solo se l'auto parla DoIP o HSFZ di BMW.

## Cosa funziona

OBD via DoIP, e le centraline via UDS di cui Caruspex conosce gli indirizzi per la marca. Gli elenchi di indirizzi per le auto DoIP sono ancora corti; le registrazioni di auto vere li riempiono (vedi [privacy](privacy.md) per cosa contiene una registrazione condivisa).

## Quale cavo?

DoIP ha bisogno delle linee Ethernet sui pin 3, 11, 12 e 13 della presa OBD e di una linea di attivazione sul pin 8. I cavi venduti per una determinata marca di solito sono cablati proprio così; il cavo ENET di BMW è uno di questi.

## Riga di comando

```
caruspex_cli enet
```

elenca i veicoli BMW e DoIP trovati in rete.
