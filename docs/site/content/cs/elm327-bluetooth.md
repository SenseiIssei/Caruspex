# ELM327 přes Bluetooth

Malé adaptéry bez kabelu. Bluetooth existuje ve dvou druzích a Caruspex umí oba.

## Bluetooth LE

Novější adaptéry jako Vgate iCar Pro BLE, vLinker, OBDLink CX a Veepeak BLE používají Bluetooth Low Energy. **Nepotřebují párování**: na stránce **Připojit** vyberte **Bluetooth**, klikněte na **Hledat zařízení Bluetooth** a vyberte adaptér ze seznamu. Funguje to ve Windows, Linuxu i macOS.

## Klasické Bluetooth

Starší a mnoho levných adaptérů používá klasické Bluetooth se sériovým profilem.

1. Jednou adaptér spárujte v nastavení Bluetooth svého systému. PIN je obvykle `1234` nebo `0000`.
2. V Caruspexu vyberte **Bluetooth** a spárovaný adaptér; seznam ukazuje, jestli je zařízení LE, klasické nebo obojí.

Ve Windows a Linuxu se Caruspex ke spárovaným klasickým adaptérům připojí přímo. Jinak Windows pro adaptér vytvoří port COM, který funguje jako u [ELM327 přes USB](elm327-usb.md). V macOS fungují klasické adaptéry Bluetooth jen tehdy, když pro ně systém nabídne sériový port; adaptéry s Bluetooth LE jsou tam lepší volba.

## Tipy

- Když adaptér nepoužíváte, vytáhněte ho: mnoho levných kusů nenechá autobaterii odpočinout.
- Pokud se adaptér neukáže, vypněte a znovu zapněte Bluetooth v počítači a zkontrolujte, že s ním není současně spojená žádná aplikace v telefonu.
- Bluetooth je pomalejší než kabel. S OBDLinkem se Caruspex ptá na několik hodnot najednou, což většinu rozdílu dožene (viz [OBDLink a STN](obdlink-stn.md)).
