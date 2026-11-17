# Kvaser (Windows)

Adaptor CAN Kvaser (Leaf, U100, Memorator, dan lainnya) bisa dipakai langsung di Windows lewat library Kvaser, CANlib. Di Linux adaptor ini menjadi antarmuka [SocketCAN](socketcan.md).

## Menyambungkan

1. Pasang driver Kvaser. Driver ini membawa `canlib32.dll`, yang dimuat oleh Caruspex.
2. Di halaman **Sambungkan** pilih **Antarmuka CAN**. Setiap kanal Kvaser yang ada perangkatnya muncul di daftar; kanal virtual dari driver tidak ditampilkan.
3. Pilih bit rate (500 kbit/s untuk OBD lewat CAN) lalu sambungkan.

## Yang bisa

OBD, setiap unit kontrol lewat UDS, pencarian unit kontrol paralel, VW TP 2.0, dan mendengarkan nilai siaran. CAN FD lewat CANlib belum dibuat.

## Baris perintah

```
caruspex_cli read --can kvaser:0@500000
```
