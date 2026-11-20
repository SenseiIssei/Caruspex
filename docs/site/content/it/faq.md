# Domande frequenti

## Caruspex costa qualcosa?

No. Caruspex è software libero con licenza MIT, senza pubblicità, account o extra a pagamento.

## Quale adattatore devo comprare?

Per la maggior parte delle persone il punto di partenza migliore è un ELM327 USB con chip FTDI o CH340, oppure un OBDLink (Bluetooth o USB). Per VW, Audi, Seat e Skoda precedenti al 2004 circa aggiungi un [cavo KKL](kkl.md); per BMW e Mini dalla serie F un [cavo ENET](enet.md). La [panoramica delle interfacce](interfaces.md) li mette a confronto.

## Funziona con la mia auto?

Qualsiasi auto con OBD-II o EOBD funziona per il motore: in Europa le benzina dal 2001 e le diesel dal 2004, negli USA dal 1996. Le auto più vecchie del gruppo VW dal 1990 circa funzionano con il cavo KKL. La pagina **Veicoli** ti dice quale strada funziona per una certa auto. Le moto da Euro 5 funzionano con un cavo adattatore per la loro presa diagnostica.

## È stato provato sulla mia auto?

Sinceramente: non ancora. Caruspex è testato contro centraline simulate; su hardware reale finora è stato verificato solo un OBDLink con il chip STN2232, senza auto. Aiutaci con un [rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Caruspex può rovinare la mia auto?

Caruspex per lo più legge. Scrive solo dopo averti chiesto conferma: la cancellazione dei codici errore e, con il cavo KKL VAG, il test attuatori e la regolazione di base della farfalla. Non codifica mai, non fa mai flash e non cambia mai i valori di adattamento. Non usarlo mentre guidi.

## Può modificare i chilometri, l'immobilizer o codificare funzioni?

No, e non lo farà. È una questione delicata dal punto di vista legale e non è lo scopo di questo strumento.

## Perché l'adattatore dice versione 2.1 ma alcune cose non vanno?

Molti adattatori economici dichiarano una versione di ELM327 più recente di quella che sono davvero. Il **Test dell'adattatore** scopre quali comandi l'adattatore conosce davvero.

## Perché i valori sono lenti sulla mia auto più vecchia?

Le auto su K-Line (in Europa circa dal 2001 al 2007) rispondono lentamente per come sono fatte. Le auto CAN sono molto più veloci.

## Caruspex invia i miei dati da qualche parte?

No. Solo il controllo degli aggiornamenti chiede a GitHub la versione più recente, e si può disattivare. Vedi [privacy](privacy.md).

## Come posso aiutare?

Invia un rapporto di prova dalla tua auto, condividi una registrazione, segnala bug o migliora una traduzione: la guida è [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), e il pulsante **Migliora la traduzione** nelle impostazioni la apre.

Anche il codice è benvenuto: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) spiega (in inglese) come compilare Caruspex e cosa serve a una pull request.
