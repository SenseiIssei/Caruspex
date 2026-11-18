# SLCAN (Lawicel)

SLCAN adalah protokol teks sederhana lewat port serial untuk stik CAN: CANable dengan firmware slcan, USBtin, Lawicel CANUSB, dan banyak perangkat rakitan sendiri. Ia berjalan di Windows, Linux, dan macOS.

## Menyambungkan

1. Sambungkan stik ke komputer dan dengan kabel ke soket OBD: CAN high di pin 6, CAN low di pin 14, ground di pin 4 dan 5.
2. Di halaman **Sambungkan** pilih **Antarmuka CAN**. Stik SLCAN ditemukan oleh pencarian port otomatis dan muncul di daftar.
3. Pilih bit rate (500 kbit/s untuk OBD lewat CAN) lalu sambungkan.

## Yang bisa

Sama seperti setiap antarmuka CAN mentah: OBD, setiap unit kontrol lewat UDS, pencarian unit kontrol paralel, VW TP 2.0, dan mendengarkan nilai siaran. CAN FD tidak bisa lewat SLCAN.

## Tips

- CANable dengan firmware candleLight bukan perangkat SLCAN: di Linux ia muncul sebagai antarmuka [SocketCAN](socketcan.md).
- Stik tidak perlu resistor terminasi di soket OBD, mobil sudah punya sendiri.

## Baris perintah

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
