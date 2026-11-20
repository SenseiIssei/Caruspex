# Privasi

Caruspex bekerja di komputermu. Tidak ada akun, tidak ada cloud, dan tidak ada pelacakan.

## Yang disimpan

Semuanya tetap di komputermu:

| Apa | Kapan | Di mana |
|---|---|---|
| Pengaturan | selalu | folder pengaturan sistemmu (tombol **Buka lokasi penyimpanan** di pengaturan menunjukkannya) |
| Garasi: VIN, mobil, pemeriksaan, catatan odometer, koneksi terakhir yang berhasil | setelah menyambungkan mobil | di sebelah pengaturan, di folder data Caruspex |
| Rekaman dan file CSV | hanya kalau kamu memulainya | tempat kamu menyimpannya |
| File log | hanya kalau kamu menyalakannya di pengaturan | jalur yang kamu pilih |
| Laporan crash | hanya kalau Caruspex crash | folder `crash-reports` di sebelah garasi |
| Laporan uji KKL pertama | hanya saat Anda menyimpannya | folder `test-reports` di folder pengaturan; VIN dan nomor imobilizer dipendekkan kecuali Anda mencentang kotaknya |

Kamu bisa menghapus file-file ini kapan saja.

## Yang dikirim

Pada setiap pemasangan, termasuk setelah memperbarui dari versi lama, Caruspex bertanya satu kali apakah perlu mencari pembaruan. Hanya setelah dijawab ya, Caruspex bertanya ke GitHub apakah ada versi yang lebih baru, saat mulai dan setiap enam jam selama berjalan. Tidak ada apa pun tentang kamu atau mobilmu yang ikut terkirim. Tanpa jawaban ya, tidak ada permintaan yang dikirim ke GitHub. Ini bisa kamu ubah kapan saja di pengaturan, di bagian **Jendela dan bahasa**. Selain itu Caruspex tidak terhubung ke internet. Komunikasi dengan adaptor Wi-Fi, kabel ENET, atau mobil DoIP tetap berada di jaringan lokalmu.

Hanya build rilis yang diterbitkan di GitHub yang mencari pembaruan; salinan yang Anda bangun sendiri tidak pernah melakukannya. Saat Caruspex mencari atau mengunduh pembaruan, GitHub melihat, seperti pada setiap unduhan, alamat IP komputer dan versi Caruspex; Caruspex sendiri tidak mengirim atau menyimpan hal lain, dan tidak ada pihak lain, termasuk proyek ini, yang melihatnya. Untuk itu berlaku pernyataan privasi GitHub. Pembaruan hanya diunduh dari rilis repositori Caruspex. Tombol seperti **Laporkan di GitHub** hanya membuka peramban Anda; apa yang Anda kirim di sana adalah keputusan Anda.

## Android

Hingga Android 11, sistem hanya membiarkan aplikasi menemukan adaptor Bluetooth LE dengan izin lokasi. Caruspex memintanya hanya karena alasan itu dan tidak pernah membaca lokasi Anda. Mulai Android 12, Caruspex meminta izin Bluetooth sebagai gantinya, yang dinyatakan tidak pernah dipakai untuk lokasi.

## Laporan crash hanya kalau kamu mau

Kalau Caruspex crash, ia menulis laporan singkat di komputermu: versi, sistem, lokasi di kode, dan rantai panggilannya. VIN, byte lalu lintas data mobil, dan folder rumahmu dibuang. Tidak ada yang dikirim. Caruspex menunjukkan laporannya dan menawarkan dua tombol: **Laporkan di GitHub** membuka issue yang sudah terisi di browser, yang bisa kamu baca dan ubah sebelum kamu kirim, **Jangan laporkan** menyingkirkannya.

## Membagikan rekaman tanpa data pribadi

Rekaman dari mobil sungguhan membantu membuat Caruspex lebih baik untuk lebih banyak mobil. **Bagikan rekaman** menyiapkan salinan dan menunjukkan dengan tepat apa yang tetap dan apa yang dibuang:

- **Dibuang**: digit terakhir VIN (di seluruh file), nama port dan alamat Bluetooth atau jaringan adaptor, serta jam perekaman.
- **Tidak pernah ada di rekaman**: lokasimu dan data pengguna.
- **Tetap**: permintaan dan jawaban, jenis adaptor, protokol, dan tanggal.

File tetap di komputermu sampai kamu sendiri menyimpannya dan melampirkannya ke issue GitHub. Tidak ada yang diunggah otomatis.

## Program pengujian

[Formulir laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) menanyakan mobil, tahun, mesin, dan adaptor. Formulir ini hanya meminta VIN yang dipersingkat; jangan pernah mengunggah VIN lengkap, pelat nomor, atau namamu.
