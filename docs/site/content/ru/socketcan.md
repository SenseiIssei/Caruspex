# SocketCAN (Linux)

SocketCAN: это CAN-подсистема ядра Linux. Через неё можно использовать любой CAN-адаптер с драйвером ядра: PEAK PCAN-USB, Kvaser, CANable с прошивкой candleLight, CAN-платы для Raspberry Pi и многие другие.

## Настройка

Поднимите интерфейс со скоростью автомобиля (500 кбит/с для OBD по CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Для CAN FD на самых новых машинах, ещё и со скоростью фазы данных:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Подключение

На странице **Подключиться** выберите **CAN-интерфейс**. Все CAN-интерфейсы компьютера есть в списке (`can0`, `can1` и так далее). Выберите нужный, задайте скорость и, для CAN FD, включите флажок CAN FD.

## Что работает

OBD, все блоки управления по UDS, параллельный поиск блоков, VW TP 2.0, прослушивание широковещательных значений и CAN FD.

## Командная строка

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
