# Caruspex

Caruspex adalah program diagnosis gratis untuk mobil dan sepeda motor. Ia berjalan di Windows, Linux, dan macOS, tersedia dalam 18 bahasa, dan bekerja dengan adaptor yang benar-benar dimiliki orang: colokan ELM327 yang murah, OBDLink, kabel KKL untuk mobil lama grup Volkswagen, kabel ENET untuk BMW, dan antarmuka CAN profesional.

![Ikhtisar dengan pengukur langsung](screenshot-overview.png)

## Apa yang dilakukannya

- Membaca dan menghapus kode kesalahan, menjelaskannya dengan kata-kata sederhana, dan memberi tahu apakah kamu masih boleh jalan terus.
- Menampilkan nilai langsung: suhu, kecepatan, campuran bahan bakar, tekanan turbo, tegangan aki, dan banyak lagi, sebagai angka dan grafik.
- Menanyai setiap unit kontrol mobil, bukan hanya mesin, sejauh mobil dan adaptornya memungkinkan.
- Memeriksa mobil bekas sebelum kamu membelinya: apakah VIN-nya cocok, apakah jarak tempuhnya sama, apakah kesalahan dihapus tak lama sebelumnya?
- Menampilkan mobil sebagai model kaca, tempat kesalahan dan nilai menyala di posisinya masing-masing.
- Memberi tahu langkah selanjutnya: berhenti sekarang, segera ke bengkel, atau bisa menunggu.

## Mulai dari mana

1. [Pasang Caruspex](install.md) di komputermu.
2. Ikuti [langkah pertama](first-steps.md): coba mobil demo, lalu sambungkan mobilmu sendiri.
3. Cari adaptormu di [adaptor dan antarmuka](interfaces.md).
4. Baca apa yang disimpan dan dikirim Caruspex di [privasi](privacy.md). Singkatnya: tidak ada yang keluar dari komputermu kecuali kamu sendiri yang membagikannya.

> Caruspex hanya membaca. Ia tidak pernah melakukan coding unit kontrol, tidak pernah mengubah jarak tempuh, dan tidak pernah menyentuh immobilizer. Menghapus kode kesalahan adalah satu-satunya hal yang ia tulis ke mobil.

## Kondisi sejujurnya

Caruspex dibuat mengikuti spesifikasi dan diuji dengan unit kontrol simulasi, gateway palsu, dan driver palsu. Di perangkat keras sungguhan sejauh ini baru satu OBDLink dengan chip STN2232 yang dicek, tanpa mobil. Kalau kamu mencoba Caruspex di mobilmu, tolong kirim [laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): setiap laporan membuat daftar mobil yang sudah diuji jadi lebih panjang.
