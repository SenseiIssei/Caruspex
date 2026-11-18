# Adaptor dan antarmuka

Caruspex mencari tahu sendiri apa yang dicolokkan dan protokol apa yang dipakai mobil. Apa pun jalurnya ke mobil, fungsi yang sama bisa dipakai: OBD-II, setiap unit kontrol lewat UDS, mendengarkan nilai siaran, merekam dan memutar ulang.

| Antarmuka | Contoh | Halaman |
|---|---|---|
| ELM327 lewat USB | kabel ELM327 murah, Vgate, Veepeak | [ELM327 lewat USB](elm327-usb.md) |
| ELM327 lewat Bluetooth | colokan Bluetooth klasik dan Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 lewat Bluetooth](elm327-bluetooth.md) |
| ELM327 lewat Wi-Fi | colokan yang membuka Wi-Fi sendiri | [ELM327 lewat Wi-Fi](elm327-wifi.md) |
| Chip STN | OBDLink SX, EX, MX+, CX | [OBDLink dan STN](obdlink-stn.md) |
| Kabel KKL (VAG 409.1) | kabel FTDI atau CP2102 | [Kabel KKL](kkl.md) |
| Kabel K+DCAN | kabel BMW dengan sakelar | [Kabel K+DCAN](kdcan.md) |
| Kabel ENET | BMW dan Mini mulai seri F | [ENET BMW](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable dengan candleLight, CAN hat Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Kabel DoIP | banyak mobil sejak sekitar 2016 | [DoIP](doip.md) |

## Apa yang bisa dilakukan masing-masing?

ELM327 biasa sudah cukup untuk mesin, transmisi, dan, di kebanyakan mobil sejak sekitar 2008, semua unit kontrol lain lewat UDS. Ia tidak bisa menangani protokol lama grup Volkswagen, KWP1281, yang mengonfirmasi setiap byte satu per satu: untuk VW, Audi, Seat, dan Skoda sebelum sekitar 2004 kamu butuh kabel KKL.

Antarmuka CAN mentah (J2534, SLCAN, SocketCAN, PCAN, Kvaser) memberi Caruspex frame CAN apa adanya. Caruspex lalu menangani protokol transportnya sendiri, sehingga pencarian unit kontrol paralel yang cepat bisa dilakukan, begitu juga VW TP 2.0 untuk mobil grup Volkswagen dari sekitar 2004 sampai 2012.

![Tes adaptor](screenshot-adapter.png)

## Apakah adaptor saya asli?

**Tes adaptor** menanyakan perintah dari setiap versi ELM327 ke adaptor, mengukur waktu responsnya, dan menyampaikan dengan jujur apa artinya: banyak adaptor murah mengaku versi 1.5 atau 2.1 tapi hanya mengenal perintah versi 1.0. Caruspex tidak pernah menulis apa pun ke adaptor.

> Yang sudah diuji sejauh ini: satu OBDLink dengan chip STN2232 lulus tes adaptor, tanpa mobil. Semua adaptor dan antarmuka lain dibuat mengikuti dokumentasinya tapi belum diuji di mobil sungguhan.
