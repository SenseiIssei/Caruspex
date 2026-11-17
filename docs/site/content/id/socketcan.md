# SocketCAN (Linux)

SocketCAN adalah lapisan CAN di kernel Linux. Setiap adaptor CAN yang punya driver kernel bisa dipakai lewat lapisan ini: PEAK PCAN-USB, Kvaser, CANable dengan firmware candleLight, CAN hat Raspberry Pi, dan banyak lagi.

## Menyiapkan

Aktifkan antarmuka dengan bit rate mobil (500 kbit/s untuk OBD lewat CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Untuk CAN FD di mobil terbaru, sekalian dengan bit rate data:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Menyambungkan

Di halaman **Sambungkan** pilih **Antarmuka CAN**. Setiap antarmuka CAN di komputer ada di daftar (`can0`, `can1`, dan seterusnya). Pilih, tentukan bit rate-nya, dan untuk CAN FD centang kotak CAN FD.

## Yang bisa

OBD, setiap unit kontrol lewat UDS, pencarian unit kontrol paralel, VW TP 2.0, mendengarkan nilai siaran, dan CAN FD.

## Baris perintah

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
