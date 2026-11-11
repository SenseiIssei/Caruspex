# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · **Bahasa Indonesia**
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Diagnostik kendaraan sumber terbuka untuk Windows, Linux, dan macOS, ditulis dengan Rust dan antarmuka Tauri 2.
Satu kabel atau dongle, satu aplikasi: membaca dan menghapus kode kesalahan, melihat data sensor langsung,
memeriksa kesiapan uji emisi, dan berkomunikasi dengan unit kontrol VW/Audi pada tingkat byte lewat kabel K-Line.

Namanya berasal dari *haruspex*, peramal Romawi yang membaca pertanda dari isi perut hewan. Caruspex membaca isi perut mobil.

Aplikasi ini berbicara dalam 18 bahasa, baik antarmuka maupun setiap teks kode kesalahan: Inggris, Jerman, Spanyol,
Prancis, Portugis, Italia, Belanda, Polandia, Ceko, Rusia, Ukraina, Turki, Arab (kanan ke kiri), Tionghoa,
Jepang, Korea, Hindi, dan Indonesia. Aplikasi mengikuti bahasa sistem, bisa diganti di pengaturan, atau
dipaksa untuk satu kali jalan dengan `--language fr` dan seterusnya. Cara terjemahan dibuat dan diperiksa ada di
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Ikhtisar dengan pengukur langsung](docs/screenshot-overview.png)

## Keunggulan

- **Dua tumpukan diagnostik lengkap**: OBD-II/EOBD standar lewat adaptor ELM327 apa pun, dan diagnostik pabrikan
  VW/Audi (KWP1281 dan KWP2000) lewat kabel KKL K-Line mentah.
- **Protokol dibuat dari nol**: KWP1281 mengonfirmasi setiap byte dengan komplemen satunya,
  sesuatu yang tidak bisa dilakukan firmware ELM327. Caruspex menggerakkan K-Line sendiri, termasuk alamat
  bangun 5 baud yang dikirim bit demi bit dengan mengalihkan sinyal break serial, dengan pewaktuan berbasis tenggat, sehingga
  jitter penjadwalan sistem operasi tidak menumpuk sepanjang kesepuluh bit itu.
- **Deteksi protokol otomatis**: setelah bangun, dua byte kunci dari ECU menunjukkan apakah ia berbicara
  KWP1281 atau KWP2000 (ISO 14230); Caruspex beralih sesuai hasilnya.
- **Workspace Rust modular**: satu crate untuk tiap bidang (tautan adaptor, protokol, jaringan dan CAN, kendaraan, data,
  rekaman, analisis, mobil demo, alat baris perintah), dan aplikasi Tauri di atasnya. Inti tidak tahu apa-apa
  tentang antarmuka, jadi aplikasi dan alat baris perintah berjalan dengan kode yang persis sama.
- **Diuji tanpa mobil**: ECU simulasi (`crates/caruspex-protocols/tests/common/`) mereproduksi
  jabat tangan byte demi byte, gateway palsu menggantikan ENET dan DoIP, driver J2534 palsu mencakup jalur PassThru,
  dan simulator ELM327 berbasis Python memungkinkan aplikasi dijalankan dari ujung ke ujung.
- **Satu set terjemahan**: inti dan antarmuka memakai berkas terjemahan yang sama di
  `translations/<code>/`, dalam 18 bahasa. Inti mencari teksnya berdasarkan teks bahasa Inggris, antarmuka membangun
  katalognya dari berkas yang sama.
- **Mengenali setiap mobil**: pengurai VIN dengan 365 kode pabrikan dan katalog lebih dari 2.100
  generasi model sejak 1980 memberi tahu mobil mana yang terhubung dan apa saja yang bisa dilakukan dengannya.
- **Mobil demo bawaan**: ELM327 di dalam proses dengan mesin dan transmisi di CAN, sehingga semuanya bisa dicoba
  tanpa perangkat keras.
- **Data dihasilkan, bukan diketik tangan**: sekitar 1.700 kode kesalahan, banyak di antaranya dihitung dari keluarga kode yang teratur
  (per silinder, per bank, per sensor oksigen) oleh `tools/gen_dtc.py`.

## Fitur

### Mengenal mobilnya
- Setelah terhubung, Caruspex membaca VIN dan memberi tahu mobil apa itu: merek, model, generasi, tahun model.
- Katalog lebih dari 2.100 generasi model dari 94 merek, 1980 sampai sekarang, termasuk merek yang sudah tidak ada.
- Untuk setiap generasi: konektor diagnostik yang mana, di mana letaknya, protokol apa, dan apakah OBD-II, kabel KKL, atau hanya alat milik pabrikan yang bisa dipakai.
- Pemeriksa VIN untuk VIN apa pun: pabrikan, negara, tahun model, digit pemeriksa, kode model.
- Garasi yang mengenali setiap mobil dari VIN-nya dan menyimpan riwayat pemeriksaan serta pembacaan odometernya.

![Kendaraan](docs/screenshot-vehicles.png)

### Pemeriksaan kesehatan satu tombol
- Membaca memori kesalahan, monitor kesiapan, dan tes komponen, lalu mengamati campuran, suhu, sensor oksigen, dan tegangan pengisian selama setengah menit.
- Skor 0 sampai 100 di mana setiap poin yang hilang dijelaskan dengan kata-kata sederhana, lengkap dengan langkah berikutnya.
- Siap untuk uji emisi? Monitor mana yang masih terbuka dan bagaimana cara mengemudi untuk menyelesaikannya.
- Laporan diagnosis dalam PDF, untuk dicetak atau dibawa ke bengkel.

![Pemeriksaan kesehatan](docs/screenshot-health.png)

### Memori kesalahan
- Kode tersimpan, tertunda, dan permanen; dengan beberapa unit kontrol, setiap kode menunjukkan siapa yang melaporkannya (mesin, transmisi).
- Sekitar 1.400 kode standar (P, C, B, U) ditambah kode pabrikan untuk VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru, dan lainnya. Kode P1 dibaca sesuai arti dari merek yang terdeteksi.
- Lampu lalu lintas untuk setiap kode dan pertanyaan yang pertama kali ditanyakan semua orang: bolehkah saya terus mengemudi?
- Penyebab yang umum, hal yang bisa Anda periksa sendiri, dan perkiraan kelas biaya.
- Mobil dalam tampilan tembus pandang menunjukkan letak komponen yang bermasalah.
- Penghapusan dengan peringatan yang jelas dan pembacaan ulang otomatis; ekspor CSV.

![Memori kesalahan](docs/screenshot-faults.png)

### Apa yang harus saya lakukan sekarang?
- Satu halaman yang memilah semua temuan Caruspex menjadi tiga langkah: berhenti sekarang, segera ke bengkel, bisa menunggu.
- Setiap poin disertai satu kalimat sederhana tentang alasannya, perkiraan kisaran biaya, dan catatan untuk ditunjukkan di bengkel (kode, unit kontrol, area, penyebab umum).
- OBD tidak bisa melihat apakah lampu mesin berkedip atau lampu peringatan merah menyala: dua kotak centang menanyakannya, dan lampu berkedip disertai misfire memindahkan poin ke "Berhenti sekarang".

![Apa yang harus saya lakukan sekarang](docs/screenshot-todo.png)

### Periksa mobil bekas
- Untuk melihat mobil sebelum membeli: apakah VIN pada mobil cocok dengan surat-surat dan katalog, apakah jarak tempuh sama di antara unit kontrol, dan apakah kesalahan baru saja dihapus (jarak dan waktu sejak penghapusan).
- Monitor kesiapan, kesalahan tersimpan, pemeriksaan kondisi, dan pada mobil listrik status baterai.
- Lampu lalu lintas, pertanyaan untuk penjual, dan PDF untuk pembelian.

![Periksa mobil bekas](docs/screenshot-used-car.png)

### Tampilan tembus 3D
- Mobil yang tersambung sebagai model kaca yang bisa diputar: mesin atau motor listrik, girboks, tangki atau baterai tegangan tinggi, knalpot, radiator, aki 12 V, dan soket OBD, ditata menurut bentuk bodi dan penggerak mobil yang terdeteksi.
- Setiap unit kontrol yang ditemukan pemindaian berada di tempat yang biasanya, dihubungkan oleh jalur datanya (CAN penggerak dan CAN kenyamanan); saat Caruspex membaca, paket-paket kecil bergerak ke unit yang ditanya.
- Kesalahan menyalakan areanya dan mewarnai unit kontrol yang bertanggung jawab menjadi merah, nilai langsung ditampilkan di bagian yang bersangkutan, dan "Periksa mobil" menerbangkan kamera ke setiap temuan. Lapisan bisa dimatikan satu per satu.
- Ukuran umum menurut bentuk bodi dan zamannya, bukan tiruan persis model tertentu: data desain dan diagram kabel pabrikan tidak tersedia bebas.

![Tampilan tembus](docs/screenshot-xray.png)

### Data langsung
- Setiap nilai mode 01 dari SAE J1979 sampai PID C4: 145 PID dengan lebih dari 300 pembacaan, dikelompokkan dan bisa dicari.
- Odometer, gigi, suhu oli, laju bahan bakar, torsi, boost, suhu gas buang, filter partikel, AdBlue, NOx, dan baterai hybrid, sejauh dilaporkan oleh mobil.
- Grafik bergulir dan perekaman CSV.

![Data langsung](docs/screenshot-live.png)

### Mengukur
- Komputer perjalanan: konsumsi bahan bakar langsung dan rata-rata, biaya, CO2, jarak, dan skor gaya berkendara yang halus, bahkan untuk mobil tanpa komputer perjalanan sendiri.
- Performa: 0 sampai 100 km/jam, 80 sampai 120, seperempat mil, dan jarak pengereman dari sinyal kecepatan (hanya di jalan yang ditutup).
- Aki dan alternator: tegangan istirahat, penurunan saat starter, tegangan pengisian dengan dan tanpa beban, dipandu dalam empat langkah.

### Pemeriksaan
- Monitor kesiapan dengan kata-kata sederhana, freeze frame, VIN.
- Pemantauan komponen mode 06 dengan nilai dan batas, jumlah misfire per silinder.
- Nama ECU, ID kalibrasi, dan checksum dari mode 09, per unit kontrol.
- Terminal ELM327 mentah.

### Tes adaptor
- ELM327 asli atau tiruan? Ia menanyakan perintah dari tiap versi, mengukur waktu respons, dan mengatakan dengan jujur apa yang bisa disimpulkan dari situ.
- Mengenali OBDLink dan adaptor STN lainnya.
- Tidak pernah menulis apa pun ke adaptor.

![Tes adaptor](docs/screenshot-adapter.png)

### Setiap kabel, setiap jalan masuk ke mobil
Caruspex mencari tahu sendiri apa yang terpasang dan bahasa apa yang dipakai mobil.

| Antarmuka | Contoh | Cara |
|---|---|---|
| ELM327 lewat USB, Bluetooth, Bluetooth LE, Wi-Fi | ELM327 murah, Vgate, vLinker, Veepeak | port serial, BLE yang dipasangkan atau langsung, TCP |
| Chip STN | OBDLink SX, EX, MX+, CX | seperti ELM327, dengan pewaktuan lebih cepat |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A dalam mode J2534 | driver 64-bit dari pabrikan, CAN dengan ISO-TP ditangani Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | port serial, ditemukan oleh pencarian port otomatis |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable dengan candleLight, Raspberry Pi CAN hat | can0 dan sejenisnya |
| Kabel KKL (VAG 409.1) | Kabel FTDI atau CP2102 | K-Line, KWP1281 dan KWP2000 |
| Kabel ENET | BMW dan Mini dari seri F | HSFZ lewat Ethernet |
| Kabel DoIP | banyak mobil sejak sekitar 2016 | ISO 13400 lewat Ethernet |

Pada semuanya, fungsi yang sama bisa dipakai: OBD-II dan OBD di atas UDS, setiap unit kontrol lewat UDS, pengalamatan
BMW, mendengarkan nilai siaran (Tesla), perekaman dan pemutaran ulang.

![Sambungkan](docs/screenshot-connect.png)

### Semua unit kontrol lewat CAN (UDS)
- Dengan ELM327 biasa pun, Caruspex menanyai setiap unit kontrol satu per satu: mesin, transmisi, ABS, airbag, panel instrumen, AC, dan lainnya, pada sebagian besar mobil sejak sekitar 2008.
- Memori kesalahan per unit beserta jenis kegagalan (korsleting, sirkuit terbuka, tidak masuk akal ...), nomor komponen, versi perangkat lunak dan perangkat keras.
- Daftar alamat untuk VAG, BMW dan Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault, dan PSA; untuk yang lain, penyisiran alamat yang lazim.
- BMW dan Mini dari seri F ke atas dengan pengalamatan diperluas milik BMW lewat 6F1, ditambah penyisiran semua 128 alamat diagnostik; kode kesalahan dalam format BMW seperti yang ditampilkan ISTA (misalnya 480A10).
- Penghapusan per unit, dengan peringatan.

![Semua unit kontrol](docs/screenshot-ecus.png)

### VW dan Audi lewat kabel KKL (KWP1281 dan KWP2000)
- Adaptasi throttle body: pengaturan dasar yang sebenarnya di blok pengukuran 060 atau 098.
- Pindai setiap unit kontrol, termasuk ABS, airbag, panel instrumen, dan kenyamanan, dengan laporan yang bisa dicetak.
- Kanal adaptasi dan tes output, sengaja hanya-baca.

### Nilai pabrikan dan nilai kustom
- Baterai mobil listrik: kondisi kesehatan, pengisian, tegangan sel dengan sebarannya, suhu, untuk Hyundai Kona dan Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf, dan Tesla Model 3 dan Y (dengan mendengarkan bus CAN); setiap nilai dengan sumber yang terdokumentasi.
- Nilai pabrikan untuk sekitar 550 kendaraan lagi dari proyek [OBDb](https://github.com/OBDb) (baterai, tekanan ban, oli, odometer, dan lainnya), ditawarkan otomatis untuk mobil yang terdeteksi; setiap persamaan diperiksa dengan dekoder milik OBDb sendiri.
- Muat nilai Anda sendiri dalam format CSV Torque yang banyak dipakai, dengan pengevaluasi lengkap untuk persamaan Torque.

![Baterai EV](docs/screenshot-battery.png)

### Rekam dan putar ulang
- Rekam satu sesi dengan mobil ke dalam berkas dan putar ulang nanti tanpa mobil, berguna untuk mencari masalah.

### Tanpa mobil atau adaptor
- Mobil demo bawaan: ia melaju, memanas, punya beberapa kesalahan tersimpan, dan berbicara seperti ELM327 sungguhan lewat CAN.
- Kamus offline semua kode, bisa disaring menurut merek.

### Tampilan dan nuansa
- Antarmuka baru bergaya Odysync: tenang, gelap, dengan transisi yang halus.
- Penyambungan langkah demi langkah dengan pencarian otomatis di semua port, dan bila gagal ada daftar periksa singkat untuk jenis sambungan itu (driver, pemasangan, kunci kontak, bit rate, adaptor tiruan) beserta yang sudah diketahui Caruspex.
- Mode Pemula dan Profesional: pemula melihat halaman sehari-hari, alat bengkel (KKL, adaptasi, perintah mentah, alamat) muncul di mode Profesional.
- 18 bahasa; pengaturan langsung berlaku.
- Memberi tahu Anda saat ada versi baru: saat dimulai ia menanyakan rilis terbaru ke GitHub dan tidak ada yang lain (tidak ada data
  tentang Anda atau mobil); bisa dimatikan di pengaturan.

## Kendaraan yang didukung

Setiap kendaraan dengan OBD-II atau EOBD: di Eropa mobil bensin sejak 2001 dan diesel sejak 2004, di AS sejak 1996. VW, Audi, Seat, dan Skoda yang lebih tua sejak sekitar 1990 bisa dipakai lewat kabel KKL. Halaman Kendaraan memberi tahu cara mana yang bisa dipakai untuk mobil tertentu.

| Era | Bus | Contoh |
|---|---|---|
| sampai sekitar 1995 | khusus pabrikan | VAG dengan KKL lewat colokan 2x2, selain itu hanya alat bengkel |
| 1996 sampai sekitar 2003 | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| sejak sekitar 2004 | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, mobil Jepang modern |
| AS sampai sekitar 2005 | SAE J1850 PWM dan VPW | Ford, Chevrolet |
| mobil terbaru | OBD di atas UDS (SAE J1979-2) | wajib di AS sejak tahun model 2027, sudah ada di beberapa mobil yang lebih baru; terdeteksi dan diterjemahkan otomatis |
| BMW dan Mini dari seri F | ENET (HSFZ lewat Ethernet) atau D-CAN dengan pengalamatan 6F1 | setiap unit kontrol |
| banyak mobil sejak sekitar 2016 | DoIP (ISO 13400) dengan kabel DoIP | OBD lewat Ethernet, ditemukan otomatis |
| Tesla Model 3 dan Y | tanpa OBD: frame CAN siaran | baterai, sel, suhu, kecepatan, odometer dengan mendengarkan (kabel adaptor ke konektor diagnostik) |

Mobil listrik murni tidak wajib menyediakan data emisi OBD-II. Sebagian tetap menjawab sebagian; untuk mobil yang tidak menjawab sama sekali tetapi ada lalu lintas di bus, Caruspex tersambung untuk mendengarkan dan membaca apa yang diketahui paket bawaan (sejauh ini Tesla Model 3 dan Y).

## Adaptor yang didukung

- Adaptor apa pun yang kompatibel dengan **ELM327**: USB, Bluetooth klasik (langsung lewat RFCOMM di Windows dan Linux setelah dipasangkan, atau lewat port COM-nya), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE, dan adaptor lain dengan layanan GATT yang lazim; ditemukan lewat
  pemindaian, tanpa pemasangan) atau Wi-Fi lewat TCP
- **Kabel ENET** (OBD ke RJ45) untuk BMW dan Mini dari seri F ke atas: Caruspex menemukan mobil di jaringan
  (HSFZ, port 6811 dan 6801) dan membaca setiap unit kontrol lewat Ethernet, tanpa chip adaptor di tengah. Komputer
  memerlukan alamat otomatis (169.254.x.x), yang diberikan sendiri oleh Windows, macOS, dan Linux saat tidak ada DHCP yang menjawab.
  Di sini pun Caruspex hanya membaca; pengodean dan flashing tetap untuk E-Sys, ISTA, atau alat tuning.
- Kabel **KKL (VAG 409.1)** apa pun. Kabel berbasis FTDI atau CP2102 jauh lebih andal di K-Line daripada
  tiruan termurah; untuk KWP1281 disarankan FTDI FT232RL asli.

Antarmuka berhak milik seperti Delphi DS150E, Autocom, atau VAS 5054A tidak didukung, karena tidak berbicara ELM327.

| Fungsi | ELM327 | KKL 409.1 |
|---|---|---|
| Membaca dan menghapus kesalahan mesin | ya | ya |
| Data langsung, freeze frame, kesiapan | ya | ya |
| ABS, airbag, panel instrumen, kenyamanan | **tidak** | ya |
| Blok pengukuran pabrikan | **tidak** | ya |
| **Adaptasi throttle body** | **tidak** | ya |

Ini batasan perangkat keras, bukan perangkat lunak: KWP1281 adalah protokol blok di mana setiap byte
dikonfirmasi oleh pihak lain. Firmware ELM327 hanya menangani pasangan permintaan/respons yang utuh.
Kabel KKL meneruskan K-Line apa adanya, sehingga Caruspex bisa berbicara protokolnya sendiri.

### Dua protokol VAG, terdeteksi otomatis

| Protokol | Khas untuk | Pengaturan dasar |
|---|---|---|
| KWP1281 (protokol blok) | mesin 1.4 MPI | perintah khusus (blok 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI dengan Bosch MED | sesi diagnostik khusus, lalu baca blok |

Kanal adaptasi dan tes output saat ini diimplementasikan di jalur KWP1281; di KWP2000
aplikasi mengatakannya terus terang, bukan gagal diam-diam.

Belum diimplementasikan: diagnostik pabrikan lewat CAN dengan VW TP 2.0 (kendaraan sejak sekitar 2004).

## Teknologi

| Bidang | Teknologi |
|---|---|
| Bahasa | Rust (edisi 2024) |
| Antarmuka | Tauri 2 dengan React 19 dan TypeScript, dibangun dengan Vite |
| Pustaka | `serialport` (port serial), `btleplug` (Bluetooth LE), `libloading` (driver J2534) |
| Lokalisasi | Satu set berkas JSON di `translations/` untuk inti dan antarmuka, 18 bahasa |
| Build | Workspace Cargo, npm untuk antarmuka |
| Tes | `cargo test` terhadap ECU simulasi, gateway palsu, dan driver J2534 palsu |
| Perkakas | Python 3 (generator kode dan tabel, pemeriksaan terjemahan, simulator ELM327) |
| Platform | Windows 10 dan 11, Linux, macOS |

## Unduh

Paket siap pakai ada di [halaman rilis](https://github.com/SenseiIssei/Caruspex/releases):

| Sistem | Berkas | Catatan |
|---|---|---|
| Windows 10 dan 11, 64-bit | `Caruspex-<version>-windows-x64-setup.exe` | Terpasang tanpa hak admin dan bisa menaruh `caruspex_cli` di jalur pencarian. Ia memperbarui 0.8 yang terpasang di tempat dan memasang runtime WebView2 hanya jika belum ada |
| Windows 10 dan 11, 64-bit | `Caruspex-<version>-windows-x64.zip` | Portabel, ekstrak lalu jalankan. Memerlukan runtime WebView2, yang sudah ada di Windows 10 dan 11 |
| macOS 13 ke atas, Apple Silicon | `Caruspex-<version>-macos.dmg` | Seret aplikasi ke Applications |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` lalu jalankan |
| Debian dan Ubuntu, 64-bit | `Caruspex-<version>-linux-amd64.deb` | Pasang dengan `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Hanya alat baris perintah | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Untuk skrip dan server, tanpa antarmuka |

Paket belum ditandatangani. Windows SmartScreen akan bertanya saat pertama kali dijalankan ("Informasi lebih lanjut", "Tetap jalankan"); di macOS
buka aplikasi pertama kali dengan klik kanan dan "Buka".

## Membangun

Persyaratan: toolchain Rust stabil (edisi 2024, Rust 1.85 atau lebih baru), Node 24 dengan npm, dan Python 3 untuk
generator dan pemeriksaan terjemahan. Di Linux diperlukan paket sistem yang dibutuhkan antarmuka dan tautan adaptor
(nama di Debian dan Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows memerlukan runtime WebView2 (bagian dari Windows 10 dan 11), macOS tidak memerlukan tambahan apa pun.

Pasang dependensi antarmuka sekali saja, lalu bangun alat baris perintah dan aplikasinya:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` menjalankan aplikasi dengan live reload selagi Anda mengerjakannya. Antarmuka
saja, tanpa jendela Tauri, berjalan di peramban biasa dengan mobil demo bawaan:

```bash
npm --prefix app run dev
```

Buka `http://localhost:5287/?demo` untuk langsung tersambung ke mobil demo. `?lang=de` memilih bahasa dan
`?theme=light` memilih tema terang, misalnya `?demo&lang=de&theme=light`. Tanpa shell Tauri tidak ada
akses ke adaptor, jadi versi peramban hanya mengenal mobil demo.

### Izin port serial di Linux

Agar bisa mengakses adaptor tanpa `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Lalu keluar dan masuk kembali.

## Hasil build

| Hasil | Keterangan |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | Aplikasi dengan antarmuka Tauri (`tauri build` juga menulis installer dan bundel ke `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Alat baris perintah yang bisa diskrip di atas inti yang sama, lihat [Alat baris perintah](#alat-baris-perintah) |

### Paket dan rilis

`bash tools/package.sh` membangun paket untuk sistem saat ini ke dalam `dist/`: installer dan dua zip di
Windows (installer memerlukan [Inno Setup 6](https://jrsoftware.org/isinfo.php)), dmg dan arsip baris perintah di
macOS, AppImage, `.deb`, dan arsip baris perintah di Linux. Lalu `bash tools/smoke_test.sh` memasang atau
membukanya seperti pengguna, menjalankan kedua program, dan menghapusnya lagi. CI menjalankan keduanya di setiap putaran.

Rilis berasal dari sebuah tag. `bash tools/release.sh <version>` menyiapkannya di cabangnya sendiri tanpa mendorong apa pun:
ia mengatur versi di `Cargo.toml` dan `app/package.json`, membuat `changelog/<version>.md` (Jerman dan Inggris),
menjalankan pemeriksaan, tes, pembuatan paket, dan smoke test, serta menyegarkan tangkapan layar README (`tools/screenshots.sh de` dan
`tools/screenshots.sh en`). Setelah pull request digabung, beri tag `v<version>` dan dorong tag itu. CI membangun ketiga
sistem dan menerbitkan rilis beserta paket dan daftar perubahannya.

CI (`.github/workflows/build.yml`) pertama-tama menjalankan pemformatan, pemeriksaan tabel yang dihasilkan, pemeriksaan terjemahan,
build antarmuka, dan clippy di Linux, lalu menguji, mengemas, dan men-smoke-test Windows, Linux, dan macOS.
Alur kerja kedua (`.github/workflows/data-check.yml`) sebulan sekali mencari kode VIN, kendaraan, dan berkas DBC baru
di hulu dan membuka issue berisi apa yang baru.

## Tes

```bash
cargo test --workspace
```

Sekitar 370 tes berjalan dalam beberapa detik, semuanya tanpa mobil. Tes-tes itu mencakup pengurai respons ELM327, penguraian
kode kesalahan, konversi PID, profil kendaraan dan pengurai VIN, layanan OBD-II dan UDS terhadap mobil demo,
sesi KWP1281 dan KWP2000 penuh terhadap unit kontrol simulasi, ENET (HSFZ) dan DoIP terhadap gateway palsu,
SLCAN dan jalur J2534 terhadap driver palsu, rekaman dan pemutaran ulang, serta analisis (pemeriksaan kesehatan, tes aki,
komputer perjalanan, pengatur waktu performa). Tes baris perintah menjalankan `caruspex_cli` yang sudah dibangun dan membandingkan keluarannya dengan
berkas golden di `crates/caruspex-cli/tests/golden/`, yang berisi apa yang dicetak versi C++ sebelumnya untuk perintah
yang sama, sehingga setiap perbedaan dengan alat lama langsung terlihat.

Pemeriksaan yang dijalankan CI selain tes:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

Dua yang pertama adalah pemformatan dan lint. `tools/i18n_rust.py --strict` gagal bila ada teks di inti yang tidak ada di
berkas terjemahan atau ada terjemahan yang hilang di salah satu dari 18 bahasa; build antarmuka memeriksa teksnya
sendiri dengan cara yang sama bila `I18N_STRICT=1` diatur. Jalankan `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py`, dan
`gen_pids.py` untuk menyegarkan tabel yang dihasilkan; CI gagal bila hasilnya berbeda dari yang sudah di-commit.

## Coba tanpa mobil

Cara tercepat adalah mobil demo bawaan: pilih "Mobil demo" di halaman Sambungkan, atau mulai dengan

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` mencantumkan setiap opsi aplikasi: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot`, dan `--size`.

Simulator Python memainkan mesin Audi A2 di K-Line atau, dengan `--can`, mobil CAN dengan mesin
dan transmisi (header, ISO-TP, mode 06 dan 09 per unit kontrol):

```bash
python tools/mock_elm327.py --can
```

Lalu sambungkan aplikasinya (di Windows berkas binernya `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

Alat baris perintah juga bekerja terhadap simulator:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Alat baris perintah

`caruspex_cli <command>` dengan `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC`, atau `--replay FILE`:

| Perintah | Fungsinya |
|---|---|
| `read` | Lampu peringatan, kesiapan, dan kode kesalahan, dengan arti menurut merek bila VIN mengungkapkannya |
| `vin` | VIN, apa yang dikatakannya (pabrikan, negara, tahun model, digit pemeriksa) dan mobil yang cocok dari katalog |
| `freeze` | Setiap freeze frame, satu per kesalahan yang tersimpan |
| `health` | Pemeriksaan kesehatan: kesalahan, kesiapan, mode 06, dan deret data langsung (`--seconds N`, bawaan 15) |
| `probe` | Tes adaptor: ELM327 itu sebenarnya apa, perintah mana yang dikenalnya, waktu bolak-balik |
| `uds` | UDS di CAN: identifikasi dan memori kesalahan setiap unit kontrol (`--ids A,B,...` untuk alamat Anda sendiri) |
| `custom` | Nilai pabrikan dan kustom dari paket bawaan (`--pack NAME`) atau CSV Torque (`--csv FILE`), sekali |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Nilai langsung, VIN dan freeze frame pertama, penghapusan, semuanya per unit kontrol, port serial, uji mandiri kabel KKL |
| `enet`, `interfaces` | Mencari mobil BMW dan Mini serta mobil DoIP di jaringan; mencantumkan antarmuka CAN di komputer ini (driver J2534, SocketCAN) |
| `version`, `help` | Versi program, daftar lengkap perintah dan opsi |

`--json` mencetak `read`, `vin`, `health`, `probe`, `freeze`, `uds`, dan `custom` sebagai JSON untuk skrip, `--record FILE`
menyimpan seluruh sesi dengan adaptor untuk `--replay`, `--fast-timing` mengalihkan adaptor ke pewaktuan
adaptif yang agresif, `-v` mencatat setiap perintah adaptor (ke stderr bila bersama `--json`). `caruspex_cli help` mencantumkan semuanya.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Tangkapan layar di halaman ini berasal dari mobil demo, diambil oleh aplikasi sendiri:
`--demo --page faults --shot shot.png --size 1440x920` membuka sebuah halaman, menyimpan gambar jendela setelah
beberapa detik, lalu keluar (`tools/screenshots.sh` mengambil seluruh set).

## Struktur proyek

```
crates/caruspex-i18n/       Bahasa: Jerman dan Inggris di dalam kode, 16 lainnya dari translations/
crates/caruspex-vehicles/   Pengurai VIN dan katalog kendaraan (merek, model, generasi, konektor, bus)
crates/caruspex-data/       Kode kesalahan, PID, alamat unit kontrol, pemetaan kode ke komponen
crates/caruspex-link/       Jalur ke adaptor (serial, TCP, Bluetooth LE), driver ELM327, tes adaptor
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 dan KWP1281 (K-Line, byte demi byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Rekaman dan pemutaran ulang, nilai kustom (CSV Torque, paket PID), penulis JSON
crates/caruspex-analysis/   Pemeriksaan kesehatan, panduan siklus berkendara, komputer perjalanan, pengatur waktu performa, tes aki
crates/caruspex-demo/       Mobil demo bawaan: ELM327 dengan unit kontrol simulasi
crates/caruspex-cli/        Alat baris perintah, dengan tes berkas golden-nya
app/src/                    Antarmuka: React 19 dan TypeScript (halaman, shell, state, laporan, i18n)
app/src-tauri/              Aplikasi Tauri: jendela, perintah, pemeriksaan pembaruan, opsi baris perintah
translations/               Berkas terjemahan bersama untuk inti dan antarmuka, 18 bahasa
data/                       Katalog kendaraan sebagai CSV yang dirawat tangan, sumber unit kontrol dan PID
tools/                      Generator, pemeriksaan terjemahan, pengemasan, skrip rilis dan tangkapan layar, simulator ELM327
docs/                       Tangkapan layar untuk README
packaging/                  Skrip installer Windows, berkas desktop Linux, Info.plist macOS
changelog/                  Catatan rilis per versi, Jerman dan Inggris
```

Basis data kode kesalahan dihasilkan: `tools/gen_dtc.py` memperluas keluarga kode EOBD yang teratur dan
daftar pilihan di `tools/dtc/` menjadi `crates/caruspex-data/src/generated/*.rs`. Jalankan ulang setelah mengubah data:

```bash
python tools/gen_dtc.py
```

Katalog kendaraan bekerja dengan cara yang sama: ubah berkas CSV di `data/vehicles/`, lalu jalankan

```bash
python tools/gen_vehicles.py
```

Alamat unit kontrol dan paket PID punya `tools/gen_ecus.py` dan `tools/gen_pids.py`. Terjemahan dijelaskan
di [translations/TRANSLATING.md](translations/TRANSLATING.md); setelah menambahkan teks di kode,
`python tools/i18n_rust.py` mencantumkan apa yang masih perlu diterjemahkan.

## Catatan tentang menghapus kode

OBD mode 04 menghapus memori kesalahan, freeze frame, dan monitor kesiapan. Tepat setelahnya
mobil akan gagal uji emisi sampai menyelesaikan beberapa siklus berkendara penuh, dan kode yang penyebabnya
masih ada akan muncul lagi setelah berkendara sebentar. Menghapus bukan memperbaiki.

## Penafian

Caruspex adalah proyek hobi dan disediakan tanpa jaminan. Gunakan dengan risiko Anda sendiri, hanya pada kendaraan
yang boleh Anda kerjakan, dan jangan pernah saat mengemudi. Semua nama produk dan merek adalah merek dagang
dari pemiliknya masing-masing; Caruspex tidak berafiliasi dengan atau didukung oleh pabrikan kendaraan mana pun.

## Lisensi

MIT, lihat [LICENSE](LICENSE). Nilai pabrikan untuk ratusan model diubah dari [OBDb](https://github.com/OBDb) dan, seperti OBDb, dilisensikan di bawah CC BY-SA 4.0: nilai itu disimpan terpisah dari kode di [data/obdb](data/obdb) bersama lisensi dan daftar sumbernya ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)).
Font yang disertakan dilisensikan di bawah SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)), dan
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
