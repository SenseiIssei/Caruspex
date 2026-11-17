# Kabel KKL (VAG 409.1)

Kabel KKL adalah kabel USB dengan chip serial dan driver K-Line, tanpa ELM327 di dalamnya. Caruspex mengendalikan K-Line sendiri, byte demi byte. Itulah yang dibutuhkan VW, Audi, Seat, dan Skoda lama: protokol mereka, KWP1281, mengonfirmasi setiap byte satu per satu, dan tidak ada ELM327 yang bisa melakukannya.

## Yang bisa

- Setiap unit kontrol VW, Audi, Seat, dan Skoda dari sekitar 1990 sampai 2004: mesin, transmisi, ABS, airbag, panel instrumen, sistem kenyamanan, dan lainnya.
- KWP1281 dan KWP2000 (ISO 14230), dikenali otomatis dari key byte setelah wake-up.
- Memori kesalahan per unit kontrol, blok pengukuran, dan setelan dasar throttle body yang sebenarnya (blok pengukuran 060 atau 098).
- Kanal adaptasi dan tes aktuator sengaja dibuat hanya-baca.

Halaman-halaman ini ada di **mode profesional**: **Unit kontrol lewat KKL**, **Throttle body**, dan **Adaptasi dan tes output**. Masing-masing membuka kabel sendiri: pilih port serial, unit kontrol, lalu mulai.

## Kabel yang mana?

Kabel dengan FTDI FT232RL atau CP2102 asli jauh lebih andal di K-Line daripada tiruan yang paling murah. Untuk KWP1281 disarankan FTDI asli. Kabel harus meneruskan K-Line apa adanya: antarmuka "HEX-USB" dengan firmware sendiri tidak bisa dipakai.

Mobil sebelum sekitar 1996 sering tidak punya soket OBD, melainkan colokan 2x2 di ruang mesin atau di bawah dasbor. Adaptor dari 2x2 ke soket 16 pin harganya murah.

## Cek kabelnya

Alat baris perintah menguji kabel tanpa mobil:

```
caruspex_cli kklcheck --port COM3
```

## Masalah yang sering terjadi

- **Unit kontrol tidak menjawab**: kontak sudah ON? Wake-up 5 baud butuh timing yang tepat; hub USB dan sebagian mesin virtual merusaknya, colokkan kabel langsung.
- **Putus setelah beberapa saat**: kabel murah kehilangan byte. Coba kabel lain sebelum kamu mencurigai mobilnya.
