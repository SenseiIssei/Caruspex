# Адаптери та інтерфейси

Caruspex сам з'ясовує, що підключено і якою мовою розмовляє авто. Хоч би яким шляхом ви підключилися до авто, працюють ті самі функції: OBD-II, усі блоки керування через UDS, прослуховування широкомовних значень, запис і відтворення.

| Інтерфейс | Приклади | Сторінка |
|---|---|---|
| ELM327 через USB | дешеві кабелі ELM327, Vgate, Veepeak | [ELM327 через USB](elm327-usb.md) |
| ELM327 через Bluetooth | адаптери з класичним Bluetooth і Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 через Bluetooth](elm327-bluetooth.md) |
| ELM327 через Wi-Fi | адаптери, що відкривають власну мережу Wi-Fi | [ELM327 через Wi-Fi](elm327-wifi.md) |
| Чипи STN | OBDLink SX, EX, MX+, CX | [OBDLink і STN](obdlink-stn.md) |
| KKL-кабель (VAG 409.1) | кабелі на FTDI або CP2102 | [KKL-кабель](kkl.md) |
| Кабель K+DCAN | кабелі для BMW з перемикачем | [Кабель K+DCAN](kdcan.md) |
| ENET-кабель | BMW і Mini починаючи з серії F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable з candleLight, CAN-плати для Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP-кабель | багато авто приблизно з 2016 року | [DoIP](doip.md) |

## Що вміє кожен із них?

Простого ELM327 досить для двигуна, коробки передач і, на більшості авто приблизно з 2008 року, усіх інших блоків керування через UDS. Він не вміє старий протокол концерну Volkswagen KWP1281, який підтверджує кожен окремий байт: для VW, Audi, Seat і Skoda приблизно до 2004 року потрібен KKL-кабель.

Прямі CAN-інтерфейси (J2534, SLCAN, SocketCAN, PCAN, Kvaser) віддають Caruspex голі CAN-кадри. Транспортний протокол він тоді веде сам, що дає змогу швидко шукати блоки керування паралельно і працювати з VW TP 2.0 на авто концерну Volkswagen приблизно з 2004 по 2012 рік.

![Тест адаптера](screenshot-adapter.png)

## Мій адаптер справжній?

**Тест адаптера** перевіряє адаптер командами кожної версії ELM327, вимірює час відповіді і чесно каже, що з цього випливає: багато дешевих адаптерів заявляють версію 1.5 або 2.1, але знають лише команди версії 1.0. Caruspex ніколи нічого не записує в адаптер.

> Перевірено на цей момент: OBDLink із чипом STN2232 пройшов тест адаптера, без автомобіля. Усі інші адаптери та інтерфейси зроблено за їхньою документацією, але ще не перевірено на справжньому авто.
