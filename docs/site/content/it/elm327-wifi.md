# ELM327 via Wi-Fi

Alcuni adattatori aprono una propria rete Wi-Fi. Il computer ci si collega e parla con l'adattatore via TCP.

## Collegamento

1. Inserisci l'adattatore e accendi il quadro. Dopo qualche secondo compare una rete Wi-Fi, spesso chiamata `WiFi_OBDII`, `V-LINK` o con il nome del produttore.
2. Collega il computer a quella rete. Non ha internet, quindi il sistema potrebbe avvisarti che la connessione è limitata: va bene così.
3. In Caruspex scegli **Wi-Fi** nella pagina **Collega**. La maggior parte degli adattatori usa l'indirizzo `192.168.0.10` e la porta `35000`; sono già impostati. Se il tuo ne usa un altro (per esempio `192.168.4.1`), scrivilo come `address:port`.

## Consigli

- Finché il computer è nella rete dell'adattatore, non ha internet. A Caruspex non importa.
- Gli adattatori Wi-Fi funzionano su ogni sistema senza driver.
- Se la connessione cade, avvicina il computer all'auto: la radio di queste spine è debole.
