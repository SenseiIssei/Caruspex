# Kabel KKL (VAG 409.1)

Kabel KKL adalah kabel USB dengan chip serial dan driver K-Line, tanpa ELM327 di dalamnya. Caruspex mengendalikan K-Line sendiri, byte demi byte. Itulah yang dibutuhkan VW, Audi, Seat, dan Skoda lama: protokol mereka, KWP1281, mengonfirmasi setiap byte satu per satu, dan tidak ada ELM327 yang bisa melakukannya.

## Yang bisa

- Setiap unit kontrol VW, Audi, Seat, dan Skoda dari sekitar 1990 sampai 2004: mesin, transmisi, ABS, airbag, panel instrumen, sistem kenyamanan, dan lainnya.
- KWP1281 dan KWP2000 (ISO 14230), dikenali otomatis dari key byte setelah wake-up.
- Memori kesalahan per unit kontrol, blok pengukuran, dan setelan dasar throttle body yang sebenarnya (blok pengukuran 060 atau 098).
- Kanal adaptasi sengaja dibuat hanya-baca. Tes aktuator menggerakkan komponen sungguhan dan baru mulai setelah Anda mengonfirmasi.

Halaman-halaman ini ada di **mode profesional**: **Unit kontrol lewat KKL**, **Throttle body**, dan **Adaptasi dan tes output**. Masing-masing membuka kabel sendiri: pilih port serial, unit kontrol, lalu mulai.

## Kabel yang mana?

Kabel dengan FTDI FT232RL atau CP2102 asli jauh lebih andal di K-Line daripada tiruan yang paling murah. Untuk KWP1281 disarankan FTDI asli. Kabel harus meneruskan K-Line apa adanya: antarmuka "HEX-USB" dengan firmware sendiri tidak bisa dipakai.

Mobil sebelum sekitar 1996 sering tidak punya soket OBD, melainkan colokan 2x2 di ruang mesin atau di bawah dasbor. Adaptor dari 2x2 ke soket 16 pin harganya murah.

## Cek kabelnya

Alat baris perintah menguji kabel tanpa mobil:

```
caruspex_cli kklcheck --port COM3
```

## Tes pertama di mobil

Halaman mode profesional **Tes pertama di mobil** memandu sesi pertama dengan kabel KKL baru. Halaman ini disesuaikan untuk Audi A2 dan bisa dipakai di VW, Audi, Seat, atau Skoda mana pun yang punya K-Line. Hanya membaca: tidak ada yang dihapus, tanpa tes output, tanpa pengaturan dasar. Port baru dibuka saat tes dimulai.

1. **Cek kabel tanpa mobil.** Colokkan kabel ke USB saja, pilih port-nya, lalu tekan **Cek kabel**. Caruspex membaca chip-nya (FTDI, CH340, Prolific, CP210x), untuk FTDI juga timer latensi, dan mengukur bit alamat 5 baud di jalur break. Tanpa mobil tidak ada echo yang kembali, dan itu benar.
2. **Setel timer latensi ke 1 ms** (kabel FTDI di Windows): Device Manager, Ports (COM & LPT), klik dua kali USB Serial Port, Port Settings, Advanced, Latency Timer (msec) ke 1, OK, lalu cabut kabel dan colokkan lagi sekali. Dengan nilai bawaan 16 ms, balasan untuk byte kunci bisa melewatkan jendela 50 ms-nya. Caruspex membaca setelan ini, tetapi tidak pernah mengubahnya.
3. **Di mobil:** kunci kontak ON, mesin mati, kabel terpasang kuat di soket OBD dan langsung ke komputer (tanpa hub USB), tidak ada program diagnosis lain yang terbuka. Centang daftar periksa lalu tekan **Mulai tes pertama**.
4. Tes membangunkan mesin (alamat 01) dan mengukur setiap bit alamat, byte sinkron 0x55, byte kunci, dan balasannya. Lalu tes membaca identifikasi, memori kesalahan, dan blok pengukuran 001 sampai 003, setelah itu panel instrumen 17, ABS 03, airbag 15, kenyamanan 46, dan kelistrikan pusat 09. Unit kontrol yang tidak menjawab mungkin memang tidak terpasang.
5. **Simpan laporan** menulis file teks dan file JSON ke folder `test-reports` di dalam folder pengaturan Caruspex. **Buka laporan tes di GitHub** mengisikan formulir laporan untuk Anda. VIN dan nomor immobilizer dipersingkat kecuali Anda mencentang kotaknya.

Arti hasilnya:

| Hasil | Yang harus dilakukan |
|---|---|
| Tidak ada echo | Kabel tidak mendengar dirinya sendiri: colokan tidak terpasang benar, atau pin 16 tidak mendapat 12 V (periksa sekring soket diagnosis). Tanpa mobil ini normal. |
| Echo-nya rusak | Ada perangkat lain di soket, sakelar kabel K+DCAN tidak di posisi K-Line, atau kabelnya rusak. |
| Timing alamat wake-up kurang presisi | Bit meleset lebih dari 20 ms dari 200 ms: jangan pakai hub USB, tutup program lain, timer latensi 1 ms. Caruspex mencoba sekali lagi dengan panggilan break yang dimajukan. |
| Kabel OK, tetapi tidak ada jawaban dari mobil | Kunci kontak sudah ON? Alamat 01 adalah mesin. Mobil sejak sekitar 2005 didiagnosis lewat CAN. |
| Balasan untuk byte kunci terlambat | Setel timer latensi ke 1 ms. |

## Masalah yang sering terjadi

- **Unit kontrol tidak menjawab**: kontak sudah ON? Wake-up 5 baud butuh timing yang tepat; hub USB dan sebagian mesin virtual merusaknya, colokkan kabel langsung.
- **Putus setelah beberapa saat**: kabel murah kehilangan byte. Coba kabel lain sebelum kamu mencurigai mobilnya.
