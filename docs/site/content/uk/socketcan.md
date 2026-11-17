# SocketCAN (Linux)

SocketCAN: це CAN-підсистема ядра Linux. Через неї можна використовувати будь-який CAN-адаптер із драйвером ядра: PEAK PCAN-USB, Kvaser, CANable з прошивкою candleLight, CAN-плати для Raspberry Pi і багато інших.

## Налаштування

Підніміть інтерфейс зі швидкістю авто (500 кбіт/с для OBD через CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Для CAN FD на найновіших авто, ще й зі швидкістю фази даних:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Підключення

На сторінці **Підключитися** виберіть **CAN-інтерфейс**. Усі CAN-інтерфейси комп'ютера є в списку (`can0`, `can1` і так далі). Виберіть потрібний, задайте швидкість і, для CAN FD, увімкніть прапорець CAN FD.

## Що працює

OBD, усі блоки керування через UDS, паралельний пошук блоків, VW TP 2.0, прослуховування широкомовних значень і CAN FD.

## Командний рядок

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
