# Assistente di collegamento e presa OBD

## Dov'è la presa?

Ogni auto venduta in Europa dal 2001 (benzina) o dal 2004 (diesel), e negli USA dal 1996, ha una presa OBD a 16 poli a portata del conducente. I posti più comuni:

- sotto il cruscotto, a sinistra o a destra del piantone dello sterzo,
- sopra i pedali,
- nel rivestimento laterale del vano piedi,
- dietro il coperchio della scatola dei fusibili,
- nella console centrale, a volte dietro un piccolo sportellino.

Per più di 200 generazioni di auto e 35 moto Caruspex conosce il punto esatto. L'assistente di collegamento lo segna su una pianta dell'abitacolo e in un primo piano del lato guida; per le moto mostra una vista laterale con la presa diagnostica e il cavo adattatore che serve.

> L'elenco delle posizioni della presa è una nostra raccolta e non è stato verificato su ogni auto. Se sulla tua il punto è sbagliato, faccelo sapere con un [rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## L'assistente di collegamento

L'assistente si apre da solo al primo avvio. Chiede quattro cose, e ogni passo si può saltare:

1. **Quale adattatore?** ELM327 con USB, Bluetooth o Wi-Fi, un cavo ENET o un cavo KKL. Le immagini aiutano a distinguerli.
2. **Quale auto?** Sceglila dal catalogo, poi l'assistente mostra dov'è la presa.
3. **Quadro acceso.** La maggior parte delle centraline risponde solo con il quadro acceso.
4. **Collega.** Caruspex cerca tra le porte e prova i protocolli.

In seguito trovi l'assistente nelle impostazioni e nella pagina Collega.

## Ricerca e memoria

Con la ricerca automatica Caruspex passa tutte le porte seriali, chiede a ognuna se c'è un ELM327 e poi prova uno dopo l'altro i protocolli dell'auto. Quando un'auto si è collegata, Caruspex si ricorda la strada (adattatore, porta, protocollo, bit rate) per il suo VIN e la prova per prima la volta successiva, così il collegamento è più veloce.

## Quando non si collega

Se nessun adattatore risponde o l'auto resta muta, Caruspex mostra una breve lista di controllo per quel tipo di collegamento, insieme a quello che ha già scoperto: driver installato, Bluetooth associato, quadro acceso, bit rate giusto, un adattatore clone che finge solo di essere una versione più recente. Le pagine in [adattatori e interfacce](interfaces.md) spiegano ogni tipo nel dettaglio.
