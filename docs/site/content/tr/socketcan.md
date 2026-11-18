# SocketCAN (Linux)

SocketCAN, Linux çekirdeğinin CAN katmanıdır. Çekirdek sürücüsü olan her CAN adaptörü bunun üzerinden kullanılabilir: PEAK PCAN-USB, Kvaser, candleLight yazılımlı CANable, Raspberry Pi CAN kartları ve daha birçoğu.

## Kurulum

Arayüzü aracın bit hızıyla etkinleştirin (CAN üzerinden OBD için 500 kbit/s):

```
sudo ip link set can0 up type can bitrate 500000
```

En yeni araçlarda CAN FD için, ayrıca bir veri bit hızıyla:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Bağlanma

**Bağlan** sayfasında **CAN arayüzü** seçin. Bilgisayarın her CAN arayüzü listededir (`can0`, `can1` vb.). Onu seçin, bit hızını belirleyin ve CAN FD için CAN FD kutusunu işaretleyin.

## Neler çalışır

OBD, UDS üzerinden her kontrol ünitesi, kontrol ünitelerinin paralel aranması, VW TP 2.0, yayınlanan değerleri dinleme ve CAN FD.

## Komut satırı

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
