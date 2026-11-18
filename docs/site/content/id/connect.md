# Asisten koneksi dan soket OBD

## Di mana soketnya?

Setiap mobil yang dijual di Eropa sejak 2001 (bensin) atau 2004 (diesel), dan di AS sejak 1996, punya soket OBD 16 pin yang bisa dijangkau dari kursi pengemudi. Tempat yang biasa:

- di bawah dasbor, di kiri atau kanan kolom setir,
- di atas pedal,
- di panel samping ruang kaki,
- di balik tutup kotak sekring,
- di konsol tengah, kadang di balik tutup kecil.

Untuk lebih dari 200 generasi mobil dan 35 sepeda motor, Caruspex tahu letak persisnya. Asisten koneksi menandainya di denah dan di gambar dekat sisi pengemudi; untuk sepeda motor, asisten menampilkan tampak samping dengan colokan diagnosis dan kabel adaptor yang dibutuhkan.

> Daftar letak soket ini kami kumpulkan sendiri dan belum dicek di setiap mobil. Kalau letaknya salah untuk mobilmu, tolong kabari kami lewat [laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Asisten koneksi

Asisten terbuka sendiri saat pertama kali dijalankan. Ia menanyakan empat hal, dan setiap langkah bisa dilewati:

1. **Adaptor yang mana?** ELM327 dengan USB, Bluetooth atau Wi-Fi, kabel ENET, atau kabel KKL. Gambar membantu membedakannya.
2. **Mobil yang mana?** Pilih dari katalog, lalu asisten menunjukkan letak soketnya.
3. **Kontak ON.** Kebanyakan unit kontrol hanya menjawab kalau kunci kontak ON.
4. **Sambungkan.** Caruspex mencari port dan mencoba protokolnya.

Nanti kamu bisa menemukan asisten ini lagi di pengaturan dan di halaman Sambungkan.

## Mencari dan mengingat

Dengan pencarian otomatis, Caruspex memeriksa semua port serial satu per satu, menanyakan apakah ada ELM327 di sana, lalu mencoba protokol mobil itu satu per satu. Begitu sebuah mobil berhasil tersambung, Caruspex mengingat caranya (adaptor, port, protokol, bit rate) untuk VIN-nya dan mencobanya lebih dulu lain kali, jadi menyambung jadi lebih cepat.

## Kalau tidak tersambung

Kalau tidak ada adaptor yang menjawab atau mobil diam saja, Caruspex menampilkan daftar periksa singkat untuk jenis koneksi itu, bersama hal-hal yang sudah ia temukan: driver terpasang, Bluetooth sudah dipasangkan, kontak ON, bit rate benar, adaptor tiruan yang cuma mengaku versi lebih baru. Halaman di bawah [adaptor dan antarmuka](interfaces.md) menjelaskan setiap jenis secara rinci.
