# ELM327 lewat USB

Colokan untuk soket OBD dengan kabel ke port USB. Ini jenis ELM327 yang paling andal: tanpa pairing, tanpa sinyal radio, tanpa baterai habis.

## Driver

Di dalam colokan ada chip USB yang mengubah kabel menjadi port serial (port COM di Windows). Driver yang dibutuhkan tergantung chip itu:

| Chip | Driver |
|---|---|
| FTDI FT232 | biasanya dipasang sendiri oleh Windows, sudah bawaan di Linux dan macOS |
| CH340 / CH341 | biasanya dipasang sendiri oleh Windows, sudah bawaan di Linux dan macOS |
| Silicon Labs CP210x | dari situs pembuatnya kalau Windows tidak menemukannya |
| Prolific PL2303 | dari situs pembuatnya; tiruan lama sering hanya jalan dengan driver lama |

Setelah dicolokkan, Device Manager di Windows menampilkan adaptor di bawah "Ports (COM & LPT)".

## Menyambungkan

1. Colokkan adaptor ke mobil, nyalakan kontak, lalu colokkan kabel ke komputer.
2. Di halaman **Sambungkan** pilih **Port USB atau COM**.
3. Biarkan port pada **Cari otomatis** atau pilih dari daftar, lalu sambungkan.

Pencarian otomatis mencoba setiap port serial pada kecepatan yang umum. Kalau kamu tahu kecepatan adaptormu, kamu bisa mengaturnya di pengaturan pada **Baud rate adaptor**.

## Linux dan macOS

Di Linux adaptor muncul sebagai `/dev/ttyUSB0` atau `/dev/ttyACM0`. Pengguna kamu harus masuk grup `dialout` (lihat [Instalasi](install.md)). Di macOS adaptor muncul sebagai `/dev/cu.usbserial-...` atau `/dev/cu.wchusbserial...`.

## Masalah yang sering terjadi

- **Tidak ada port yang muncul**: driver belum ada, atau kabelnya cuma kabel untuk mengisi daya.
- **Adaptor menjawab, mobil tidak**: kontak OFF, atau adaptornya tiruan yang tidak sanggup menangani protokol mobil. [Tes adaptor](interfaces.md) memberi tahu lebih banyak.
- **Nilai langsung sangat lambat di mobil lama**: K-Line memang lambat, itu bukan salah adaptornya.
