# ELM327 via Bluetooth

Plugues pequenos, sem cabo. Existem dois tipos de Bluetooth, e o Caruspex fala os dois.

## Bluetooth LE

Adaptadores mais novos, como Vgate iCar Pro BLE, vLinker, OBDLink CX e Veepeak BLE, usam Bluetooth Low Energy. Eles **não precisam de pareamento**: na página **Conectar**, escolha **Bluetooth**, clique em **Procurar dispositivos Bluetooth** e escolha o adaptador na lista. Isso funciona no Windows, Linux, macOS e Android.

## Bluetooth clássico

Adaptadores mais antigos e muitos baratos usam Bluetooth clássico com perfil serial.

1. Pareie o adaptador uma vez nas configurações de Bluetooth do seu sistema. O PIN normalmente é `1234` ou `0000`.
2. No Caruspex, escolha **Bluetooth** e selecione o adaptador pareado; a lista mostra se um dispositivo é LE, clássico ou os dois.

No Windows e no Linux, o Caruspex se conecta direto aos adaptadores clássicos pareados. Como alternativa, o Windows cria uma porta COM para o adaptador, que funciona como [ELM327 via USB](elm327-usb.md). No macOS, adaptadores Bluetooth clássicos só funcionam se o sistema oferecer uma porta serial para eles; lá, adaptadores com Bluetooth LE são a melhor escolha.

## No Android

- Os adaptadores Bluetooth LE também não precisam de pareamento aqui: escolha **Bluetooth** e pesquise; a lista mostra a intensidade do sinal de cada dispositivo.
- Os adaptadores clássicos são pareados uma vez nas configurações do Android e depois aparecem na lista.
- A partir do Android 12, o sistema pede uma vez acesso a dispositivos próximos. Até o Android 11, a pesquisa só encontra adaptadores Bluetooth LE se o Caruspex puder usar a localização e a localização estiver ativada; o Caruspex não a lê. Os adaptadores clássicos pareados funcionam sem ela.

## Dicas

- Tire o adaptador da tomada quando não estiver usando: muitos baratos ficam gastando a bateria do carro.
- Se o adaptador não aparecer, desligue e ligue de novo o Bluetooth do computador e confira se nenhum app de celular está conectado a ele ao mesmo tempo.
- Bluetooth é mais lento que cabo. Com um OBDLink, o Caruspex pede vários valores de uma vez, o que compensa a maior parte disso (veja [OBDLink e STN](obdlink-stn.md)).
