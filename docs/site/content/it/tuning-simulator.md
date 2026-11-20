# Simulatore di tuning

La pagina **Simulatore di tuning** (modalità pro, sotto Misura) è una BMW N55 virtuale: il sei cilindri in linea da 3,0 litri con un turbo twin-scroll della F30 335i e della F32/F36 435i, 306 CV e 400 Nm. Cambia pressione del turbo, accensione e lambda per fascia di regime e guarda cosa dice il modello, molto prima che qualcosa tocchi un'auto.

> **Solo simulazione.** Caruspex non scrive mai mappe motore e non riprogramma mai. Non conosce alcun formato di file di flash, alcuna somma di controllo né indirizzo di mappa, e non copia alcun tuning commerciale. Il vero tuning spetta a strumenti affermati e a un'officina, e la modifica va omologata (in Germania dal TÜV). Senza, l'auto perde l'omologazione.

![Simulatore di tuning](screenshot-tune.png)

## Cosa puoi cambiare

- Scostamenti in cinque fasce di regime (da 1000 a 2000, da 2000 a 3500, da 3500 a 5000, da 5000 a 6000, da 6000 a 7000): pressione del turbo in bar, accensione in gradi, lambda.
- Carburante (RON 95, 98, 100 o 102), temperatura esterna, pressione atmosferica e il cambio (automatico 8HP45 o manuale a sei marce).
- **Serie** riporta gli scostamenti a zero, **Ripristina** anche le condizioni.

La pagina mostra coppia e potenza di serie contro quelle modificate, ogni limite toccato con il suo motivo, e i valori interni: pressione del turbo, aria di sovralimentazione, accensione, margine di detonazione, temperatura dei gas di scarico, turbo, pompa ad alta pressione, pressione media effettiva.

## Come calcola il modello

1. **Aria.** L'obiettivo di pressione è limitato da quanto il turbo può dare a quel regime, dal suo rapporto di compressione e dalla sua portata. Il compressore scalda l'aria, l'intercooler ne toglie una parte del calore. Pressione, temperatura e rendimento volumetrico danno la massa d'aria.
2. **Carburante e accensione.** Il carburante deriva da aria e lambda. L'accensione è quella di serie più il tuo scostamento, ma mai più anticipata di quanto consenta il limite di detonazione: il controllo della detonazione del motore la toglierebbe. Il limite si sposta con il numero di ottano, la temperatura dell'aria, la pressione e la lambda.
3. **Potenza.** L'energia del carburante per un rendimento che cala con la distanza dall'anticipo migliore, meno gli attriti, dà coppia e potenza.
4. **Limiti.** Turbo, temperatura dei gas di scarico, pompa ad alta pressione, iniettori, preaccensione a basso regime, cambio, aria di sovralimentazione e miscela danno ciascuno un avviso in parole semplici.

Una curva intera (25 punti da 1000 a 7000 giri/min) richiede circa 0,015 ms nella versione di rilascio.

## Dati pubblicati

| Valore | Dato | Fonte |
|---|---|---|
| Cilindrata, alesaggio, corsa, compressione | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Coppia | 400 Nm da 1200 a 5000 giri/min | Wikipedia "BMW N55", dati BMW della 435i |
| Potenza | 225 kW (306 CV) a 5700 a 6000 giri/min | Wikipedia (da 5700 a 5800), BMW (da 5800 a 6000) |
| Regime massimo | 7000 giri/min | Wikipedia "BMW N55" |
| Coppia d'ingresso nominale dell'8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Ipotesi

Nessuno pubblica con licenza libera le mappe interne del N55. Questi valori sono mie ipotesi; ognuna è motivata in [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- La **pressione di serie** è ricavata a ritroso con il modello dalla curva pubblicata (circa 0,7 a 0,8 bar fino a 5000 giri/min, meno oltre). Che la curva di serie torni mostra quindi coerenza, non verità.
- Rendimento volumetrico da 0,86 a 0,97, rendimento indicato 0,39, attriti quadratici con il regime.
- Accensione limitata dalla detonazione da 3 a 15 gradi prima del punto morto superiore con 1,5 gradi di margine; si sposta di 1 grado per ottano, 0,1 grado per kelvin d'aria, 6 gradi per bar e 2,5 gradi per 0,1 di lambda.
- Turbo: rapporto di compressione fino a 2,55, portata corretta fino a 0,32 kg/s, rendimento 0,72. Efficacia dell'intercooler 0,85, meno con portata alta.
- Limite di temperatura dei gas di scarico 980 °C, pompa ad alta pressione 0,027 kg/s, finestra di iniezione del 40 % del ciclo, rischio di preaccensione oltre 18 bar di pressione media effettiva sotto i 2500 giri/min, manuale a sei marce circa 450 Nm.
- Carburante E5: rapporto aria carburante 14,4, 42,4 MJ/kg.

## Quanto è preciso?

- Serie contro la curva pubblicata: 400 Nm entro l'1 % da 1250 a 5000 giri/min, 306 CV a 6000 giri/min e 304 CV a 5750 giri/min. I test ammettono il 3 %.
- Il modello non è ancora stato verificato su un vero N55. Ogni numero è una stima: circa ±8 % su coppia e potenza, ±0,1 bar di pressione, ±10 K di aria di sovralimentazione, ±3 gradi di margine di detonazione, ±60 K di temperatura dei gas di scarico.

## Confronto con la tua auto

1. Collegati, apri **Valori live** con regime, carico, pressione nel collettore, temperatura dell'aria aspirata, anticipo di accensione, lambda e massa d'aria, e avvia una registrazione.
2. Accelera una volta a fondo su tutto l'arco di regime, solo dove è consentito e sicuro.
3. Apri la registrazione nella pagina del simulatore. Caruspex tiene i punti a pieno carico, li raggruppa in fasce di 500 giri/min e mostra misurato contro simulato: pressione nel collettore e aria di sovralimentazione con le tue mappe, massa d'aria e accensione con la pressione e la temperatura misurate.

L'OBD riporta la pressione nel collettore solo fino a 255 kPa, e quale sensore la centralina usi per la temperatura di aspirazione non è documentato. **Vedi esempio** riempie la tabella con valori inventati, per vedere come si legge.

## Buono a sapersi

- Più coppia di serie ai bassi regimi aumenta il rischio di preaccensione, che può rompere i pistoni in un colpo.
- Un'accensione più anticipata del limite di detonazione non porta nulla: il controllo della detonazione la toglie.
- Un carburante migliore da solo dà margine, non potenza: la mappa di accensione di serie è il tetto.
