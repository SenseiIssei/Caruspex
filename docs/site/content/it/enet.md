# Cavo ENET di BMW

BMW e Mini dalla serie F in poi (dal 2008 circa) hanno Ethernet sulla presa OBD. Un cavo ENET va dalla presa OBD alla porta di rete del computer, senza nessun chip adattatore in mezzo. Caruspex parla HSFZ con il gateway dell'auto (porte 6811 e 6801) e raggiunge tutte le centraline.

## Collegamento

1. Collega il cavo all'auto e alla porta di rete del computer (o a un adattatore di rete USB). Accendi il quadro.
2. Aspetta fino a un minuto: senza server DHCP, Windows, macOS e Linux danno da soli al computer un indirizzo automatico (169.254.x.x).
3. Nella pagina **Collega** scegli **Cavo di rete** e lascia vuoto il campo: Caruspex cerca l'auto in rete. Se la ricerca non trova niente, scrivi l'indirizzo dell'auto.

## Cosa funziona

- Tutte le centraline via UDS con l'indirizzamento di BMW, codici errore nel formato BMW come li mostra ISTA (per esempio 480A10).
- Memoria errori, identificazione e cancellazione per ogni centralina, valori live via OBD.
- Anche qui Caruspex legge soltanto. Codifica e flash restano ai programmi d'officina.

## Quale cavo?

Un normale cavo ENET ha una resistenza tra i pin 8 e 16 che accende l'Ethernet dell'auto. I cavi senza restano muti. Le porte di rete da 1 Gbit/s funzionano, l'auto parla a 100 Mbit/s.

## Riga di comando

```
caruspex_cli ecus --enet auto
```
