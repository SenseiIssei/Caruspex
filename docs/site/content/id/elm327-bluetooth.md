# ELM327 lewat Bluetooth

Colokan kecil tanpa kabel. Ada dua jenis Bluetooth, dan Caruspex bisa keduanya.

## Bluetooth LE

Adaptor yang lebih baru seperti Vgate iCar Pro BLE, vLinker, OBDLink CX, dan Veepeak BLE memakai Bluetooth Low Energy. Adaptor ini **tidak perlu dipasangkan**: di halaman **Sambungkan** pilih **Bluetooth**, klik **Cari perangkat Bluetooth**, lalu pilih adaptornya dari daftar. Ini berjalan di Windows, Linux, dan macOS.

## Bluetooth klasik

Adaptor lama dan banyak adaptor murah memakai Bluetooth klasik dengan profil serial.

1. Pasangkan (pairing) adaptor sekali di pengaturan Bluetooth sistemmu. PIN-nya biasanya `1234` atau `0000`.
2. Di Caruspex pilih **Bluetooth** lalu pilih adaptor yang sudah dipasangkan; daftar menunjukkan apakah perangkat itu LE, klasik, atau keduanya.

Di Windows dan Linux, Caruspex langsung tersambung ke adaptor klasik yang sudah dipasangkan. Sebagai alternatif, Windows membuat port COM untuk adaptor, yang dipakai seperti [ELM327 lewat USB](elm327-usb.md). Di macOS, adaptor Bluetooth klasik hanya bisa dipakai kalau sistem menyediakan port serial untuknya; di sana adaptor Bluetooth LE adalah pilihan yang lebih baik.

## Tips

- Cabut adaptor kalau tidak dipakai: banyak adaptor murah terus menyedot aki mobil.
- Kalau adaptor tidak muncul, matikan lalu nyalakan lagi Bluetooth di komputer, dan pastikan tidak ada aplikasi ponsel yang tersambung ke adaptor pada saat yang sama.
- Bluetooth lebih lambat daripada kabel. Dengan OBDLink, Caruspex menanyakan beberapa nilai sekaligus, dan itu menutup sebagian besar selisihnya (lihat [OBDLink dan STN](obdlink-stn.md)).
