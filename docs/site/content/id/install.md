# Instalasi

Setiap rilis ada di [halaman rilis di GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). Paketnya belum ditandatangani, jadi Windows dan macOS akan bertanya sekali sebelum pertama kali dijalankan.

## Windows 10 dan 11

| File | Isinya |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Penginstal. Tidak butuh hak admin, bisa menaruh alat baris perintah `caruspex_cli` di jalur pencarian, dan hanya memasang runtime WebView2 kalau belum ada. |
| `Caruspex-<version>-windows-x64.zip` | Portabel: ekstrak lalu jalankan `caruspex.exe`. |

Saat pertama kali dijalankan, Windows SmartScreen mungkin bilang aplikasinya tidak dikenal: klik "More info", lalu "Run anyway".

Adaptor USB butuh driver untuk chip USB-nya. Windows 10 dan 11 biasanya menemukannya sendiri; kalau tidak, lihat [ELM327 lewat USB](elm327-usb.md).

## macOS 13 dan yang lebih baru (Apple Silicon)

Unduh `Caruspex-<version>-macos.dmg`, buka, lalu seret Caruspex ke Applications. Karena aplikasinya belum dinotarisasi, buka pertama kali dengan klik kanan lalu "Open".

## Linux (x86_64)

| File | Isinya |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Jalan di hampir semua distribusi: jadikan bisa dieksekusi dengan `chmod +x` lalu jalankan. Ia juga memperbarui dirinya sendiri. |
| `Caruspex-<version>-linux-amd64.deb` | Untuk Debian dan Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Untuk memakai adaptor serial tanpa `sudo`, tambahkan dirimu ke grup `dialout` sekali, lalu keluar dan masuk lagi:

```
sudo usermod -aG dialout $USER
```

## Hanya alat baris perintah

Untuk skrip dan server ada `caruspex_cli` tanpa antarmuka, untuk Windows, Linux, dan macOS. `caruspex_cli help` menampilkan daftar perintahnya.

`caruspex_cli report` menulis laporan diagnosis sebagai berkas PDF, tanpa aplikasi dan lewat setiap koneksi yang dikenal perintah lainnya (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Laporan bisa dibuat dalam bahasa Jerman, Inggris, Spanyol, Prancis, Portugis, Italia, Belanda, dan Indonesia (`--lang`). Untuk bahasa lain, laporan keluar dalam bahasa Inggris, karena huruf bahasa itu tidak ada di fon bawaan setiap penampil PDF.

## Pembaruan

Saat dijalankan, Caruspex menanyakan ke GitHub apakah ada versi yang lebih baru, dan tidak lebih dari itu. Versi baru hanya dipasang kalau tanda tangannya cocok dengan kunci yang tertanam di aplikasi. Pemeriksaan dan pemasangan otomatis sama-sama bisa dimatikan di pengaturan.

## Pengelola paket

Paket untuk winget, Homebrew, Flathub, dan Snap sedang disiapkan. Sampai paket itu terbit, file di atas adalah cara memasang Caruspex.
