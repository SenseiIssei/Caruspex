# Keamanan dan pembaruan

Halaman ini berisi informasi keamanan Caruspex: untuk apa program ini, risiko apa yang perlu diketahui, bagaimana
pembaruan bekerja, dan cara menghapusnya. Isinya mengikuti informasi untuk pengguna yang diminta Undang-Undang
Ketahanan Siber Uni Eropa (Cyber Resilience Act, Lampiran II), sejauh berlaku untuk program bebas dan nonkomersial.

## Programnya

- **Nama**: Caruspex, aplikasi `caruspex` dan alat baris perintah `caruspex_cli`, untuk Windows, Linux, dan macOS.
  Versinya tertulis di nama setiap unduhan dan di daftar perubahan setiap rilis.
- **Diterbitkan oleh**: proyek Caruspex di [GitHub](https://github.com/SenseiIssei/Caruspex), sebagai perangkat lunak
  bebas dengan lisensi MIT. Pertanyaan dan bug masuk ke [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Tujuan**: membaca dan menjelaskan data diagnosis mobil, sepeda motor, dan truk lewat adaptor milik sendiri, di
  komputer sendiri. Tanpa akun, tanpa cloud, tanpa pelacakan ([Privasi](privacy.md)).
- **Sifat keamanan**: membaca tidak pernah mengubah apa pun di mobil, dan semua yang membuat mobil melakukan sesuatu
  bertanya dulu. Pembaruan hanya dipasang jika tanda tangannya cocok dengan kunci yang tertanam di aplikasi. Caruspex
  tidak membuka port jaringan dan tidak menjalankan kode dari berkas atau dari internet.

## Melaporkan kerentanan {#report}

Laporkan masalah keamanan secara pribadi melalui
[formulir laporan keamanan baru](https://github.com/SenseiIssei/Caruspex/security/advisories/new), bukan di issue
publik. [Kebijakan keamanan](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) menjelaskan langkah
berikutnya. Kerentanan yang sudah diperbaiki diterbitkan sebagai
[pemberitahuan keamanan](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Risiko yang perlu diketahui {#risks}

- Unduh Caruspex hanya dari [halaman rilis](https://github.com/SenseiIssei/Caruspex/releases). Salinan dari tempat
  lain bisa saja sudah diubah.
- Buka rekaman, paket PID, dan model 3D hanya dari orang yang Anda percayai. Berkas itu tidak bisa menjalankan kode,
  tetapi berkas yang rusak bisa membuat Caruspex lambat atau tertutup.
- Hapus kode kerusakan, jalankan uji aktuator dan pengaturan dasar hanya saat mobil berhenti di tempat yang aman.
- Banyak adaptor Bluetooth dan Wi-Fi tidak punya PIN atau jaringannya terbuka: siapa pun di dekat Anda bisa
  tersambung. Cabut adaptor setelah selesai.
- Kabel ENET, DoIP, dan adaptor Wi-Fi tidak mengenkripsi apa pun: gunakan secara langsung atau hanya di jaringan
  sendiri.
- Driver J2534 adalah program dari pembuatnya dan berjalan dengan hak akses Anda. Pasang hanya dari pembuatnya.

## Pembaruan dan dukungan {#support}

- Pada setiap pemasangan, termasuk setelah memperbarui dari versi lama, Caruspex bertanya satu kali "Cari pembaruan?". Hanya setelah dijawab ya, Caruspex bertanya ke GitHub apakah ada versi lebih baru, saat mulai dan setiap enam jam selama berjalan. Versi baru hanya menyalakan sebuah tombol di bilah judul dan tidak pernah mengganggu. Memasang cukup satu klik, dan aplikasi hanya memasang paket yang tanda tangannya cocok. AppImage dan aplikasi macOS memperbarui dirinya sendiri; paket .deb dan aplikasi Android mendapat halaman unduhan.
- Pencarian dan pemasangan otomatis adalah sakelar di pengaturan, di bagian **Jendela dan bahasa**, dan bisa diubah kapan saja. Tanpa jawaban ya, tidak ada permintaan yang dikirim ke GitHub; sesekali lihat halaman rilis.
- Perbaikan keamanan hanya masuk ke rilis terbaru. Versi lama tidak mendapat perbaikan tersendiri, jadi selalu
  perbarui Caruspex.
- Caruspex dirawat oleh satu orang di waktu luang, sebisa waktu mengizinkan, tanpa janji apa pun: tanpa masa dukungan dan tanpa batas waktu balasan. Jika perawatan berhenti, repositori akan diarsipkan dan menyatakannya.

## Memeriksa unduhan {#verify}

Setiap rilis punya berkas `SHA256SUMS` berisi checksum setiap berkas, serta catatan bertanda tangan GitHub tentang
cara berkas-berkas itu dibuat. Dengan semua berkas di satu folder:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Tanpa alat ini, `Get-FileHash <berkas>` di PowerShell (Windows) atau `shasum -a 256 <berkas>` (macOS) menampilkan
checksum untuk dibandingkan dengan barisnya di `SHA256SUMS`.

## Menghapus Caruspex dan data Anda {#remove}

1. Copot pemasangan program: di Windows lewat Pengaturan, Aplikasi; di macOS seret Caruspex keluar dari Aplikasi;
   di Linux hapus AppImage atau jalankan `sudo apt remove caruspex`.
2. Hapus data Anda: tombol **Buka lokasi penyimpanan** di pengaturan menunjukkan folder berisi pengaturan, garasi,
   dan laporan crash. Hapus folder itu. Rekaman dan laporan yang Anda simpan sendiri ada di tempat Anda menyimpannya.

## Untuk pengembang dan perusahaan {#integrators}

- Setiap rilis punya daftar komponen perangkat lunak per platform, `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX), berisi setiap pustaka dan versinya.
- Penilaian risiko, daftar persyaratan, dan cakupan Cyber Resilience Act untuk Caruspex ada di
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex tidak dipasarkan sebagai produk dalam arti Cyber Resilience Act, sehingga tidak ada deklarasi kesesuaian
  UE dan tidak ada tanda CE.
