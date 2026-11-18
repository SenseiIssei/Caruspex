# Kabel K+DCAN

Kabel K+DCAN adalah kabel yang umum dipakai untuk BMW dari sekitar 1998 sampai 2017 dengan program bengkel INPA dan ISTA. Di dalamnya ada chip serial FTDI dan konverter kecil untuk D-CAN BMW. Banyak kabel seperti ini punya sakelar atau jumper antara pin 7 dan 8 untuk mobil yang lebih lama.

## Kondisi di Caruspex

> Kondisi sejujurnya: Caruspex belum mendukung mode D-CAN kabel ini. Protokol BMW lama (DS2 dan KWP2000 versi BMW sendiri) lewat K-Line sudah direncanakan, tapi perlu diuji dulu di mobil sungguhan.

Yang bisa kamu pakai sebagai gantinya hari ini:

- **BMW dan Mini mulai seri F (sekitar 2008 ke atas)**: [kabel ENET](enet.md) menjangkau setiap unit kontrol lewat Ethernet, atau [ELM327](elm327-usb.md) biasa dengan pengalamatan BMW sendiri lewat CAN.
- **OBD standar di BMW dengan CAN (sekitar 2007 ke atas)**: ELM327 apa pun bisa membaca kesalahan mesin dan nilai langsung.
- **Dalam mode K-Line** kabel ini meneruskan K-Line apa adanya seperti [kabel KKL](kkl.md). Itu berguna untuk mobil grup VW; unit kontrol BMW di K-Line butuh protokol yang masih belum ada.

## Butuh bantuan

Kalau kamu punya kabel K+DCAN dan BMW lama, [laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) dengan rekaman akan membantu membangun bagian ini.
