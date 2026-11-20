# Caruspex

Caruspex è un programma di diagnosi gratuito per auto e moto. Gira su Windows, Linux e macOS, parla 18 lingue e funziona con gli adattatori che la gente possiede davvero: l'economica spina ELM327, un OBDLink, un cavo KKL per le auto più vecchie del gruppo Volkswagen, un cavo ENET per BMW e interfacce CAN professionali.

![La panoramica con gli strumenti live](screenshot-overview.png)

## Cosa fa

- Legge e cancella i codici errore, li spiega con parole semplici e ti dice se puoi continuare a guidare.
- Mostra i valori live: temperature, velocità, miscela, pressione turbo, tensione della batteria e molti altri, come numeri e come grafici.
- Interroga ogni centralina dell'auto, non solo il motore, per quanto l'auto e l'adattatore lo permettono.
- Controlla un'auto usata prima che tu la compri: il VIN corrisponde, i chilometri tornano, sono stati cancellati errori poco prima?
- Mostra l'auto come un modello di vetro in cui errori e valori si accendono al loro posto.
- Ti dice cosa fare dopo: fermarti subito, andare presto in officina, oppure può aspettare.

## Da dove cominciare

1. [Installa Caruspex](install.md) sul tuo computer.
2. Segui i [primi passi](first-steps.md): prova l'auto demo, poi collega la tua.
3. Cerca il tuo adattatore in [adattatori e interfacce](interfaces.md).
4. Leggi cosa Caruspex salva e invia in [privacy](privacy.md). In breve: niente lascia il tuo computer, a meno che non sia tu a condividerlo.

> Caruspex per lo più legge. Scrive solo dopo averti chiesto conferma: la cancellazione dei codici errore e, con il cavo KKL VAG, il test attuatori e la regolazione di base della farfalla. Non codifica mai le centraline, non cambia mai i chilometri e non tocca mai l'immobilizer.

## Lo stato reale

Caruspex è costruito secondo le specifiche e testato contro centraline simulate, gateway finti e driver finti. Su hardware reale finora è stato verificato solo un OBDLink con il chip STN2232, senza auto. Se provi Caruspex sulla tua auto, inviaci un [rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): ogni rapporto allunga l'elenco delle auto provate.
