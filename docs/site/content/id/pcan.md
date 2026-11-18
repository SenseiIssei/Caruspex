# PEAK PCAN (Windows)

Adaptor CAN dari PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro, dan lainnya) bisa dipakai langsung di Windows, tanpa driver J2534. Di Linux adaptor ini menjadi antarmuka [SocketCAN](socketcan.md).

## Menyambungkan

1. Pasang driver perangkat dari PEAK. Driver ini membawa library `PCANBasic.dll`, yang dimuat oleh Caruspex.
2. Di halaman **Sambungkan** pilih **Antarmuka CAN**. Setiap kanal PCAN yang ada perangkatnya muncul sendiri di daftar; kanal yang sedang dipakai program lain tidak ditampilkan.
3. Pilih bit rate (500 kbit/s untuk OBD lewat CAN) lalu sambungkan.

## Yang bisa

OBD, setiap unit kontrol lewat UDS, pencarian unit kontrol paralel, VW TP 2.0, dan mendengarkan nilai siaran. CAN FD lewat PCAN-Basic belum dibuat.

## Baris perintah

```
caruspex_cli read --can pcan:USB1@500000
```
