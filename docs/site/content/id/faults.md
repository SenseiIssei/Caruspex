# Kesalahan

Halaman **Memori kesalahan** membaca apa yang disimpan unit kontrol dan menjelaskannya dengan kata-kata sederhana.

![Memori kesalahan](screenshot-faults.png)

## Yang kamu lihat

- Kode **tersimpan, tertunda, dan permanen**. Kalau ada beberapa unit kontrol, setiap kode menunjukkan siapa yang melaporkannya (mesin, transmisi).
- Sekitar 9.400 kode standar (P, C, B, U) dan kode pabrikan dari VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai dan Kia, Subaru, dan lainnya. Kode P1 dibaca sesuai arti dari merek yang terdeteksi.
- **Lampu lalu lintas** per kode dan pertanyaan yang pertama kali ditanyakan semua orang: masih boleh jalan terus?
- Penyebab yang umum, hal yang bisa kamu cek sendiri, dan perkiraan kelas biaya.
- Letak komponennya: mobil dalam tampilan tembus pandang, atau [Tampilan tembus](xray.md).

## Semua unit kontrol

Dalam mode profesional, halaman **Semua unit kontrol** menanyai setiap unit kontrol, bukan hanya mesin: ABS, airbag, panel instrumen, AC, dan lainnya. Setiap kesalahan disertai jenis kerusakannya (korsleting, kabel putus, sinyal tidak masuk akal), nomor part, dan versi perangkat lunak. Mobil lama grup VW dibaca lewat [kabel KKL](kkl.md).

## Apa yang harus saya lakukan sekarang?

Halaman **Apa yang harus dilakukan?** memilah semua temuan Caruspex ke dalam tiga tingkat: berhenti sekarang, segera ke bengkel, bisa menunggu. Setiap poin punya satu kalimat sederhana tentang alasannya, perkiraan kisaran biaya, dan catatan untuk ditunjukkan di bengkel. OBD tidak bisa melihat apakah lampu check engine berkedip atau ada lampu peringatan merah yang menyala, jadi dua kotak centang menanyakannya padamu; lampu yang berkedip disertai misfire memindahkan poin itu ke "berhenti sekarang".

![Apa yang harus saya lakukan sekarang](screenshot-todo.png)

## Menghapus kode

Penghapusan bisa dilakukan per unit kontrol, dengan peringatan yang jelas, dan Caruspex langsung membaca memorinya lagi sesudahnya.

> Menghapus bukan memperbaiki. Penghapusan juga mereset monitor kesiapan: sampai mobil menyelesaikan beberapa siklus berkendara penuh, mobil akan gagal uji emisi, dan kode yang penyebabnya masih ada akan muncul lagi setelah jalan sebentar.

## Ekspor

Daftarnya bisa disimpan sebagai CSV, dan laporan inspeksi sebagai PDF.
