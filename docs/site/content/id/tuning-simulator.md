# Simulator tuning

Halaman **Simulator tuning** (mode pro, di bawah Ukur) adalah BMW N55 virtual: mesin enam silinder segaris 3,0 liter dengan satu turbo twin-scroll dari F30 335i dan F32/F36 435i, 306 PS dan 400 Nm. Ubah tekanan turbo, pengapian, dan lambda per rentang putaran dan lihat apa kata model, jauh sebelum apa pun menyentuh mobil.

> **Hanya simulasi.** Caruspex tidak pernah menulis peta mesin dan tidak pernah melakukan flashing. Ia tidak mengenal format berkas flash, checksum, atau alamat peta, dan tidak menyalin tuning komersial. Tuning sungguhan dilakukan dengan alat yang sudah mapan dan di bengkel, dan perubahannya perlu disetujui (di Jerman oleh TÜV). Tanpa itu mobil kehilangan izin jalannya.

![Simulator tuning](screenshot-tune.png)

## Yang bisa diubah

- Selisih di lima rentang putaran (1000 sampai 2000, 2000 sampai 3500, 3500 sampai 5000, 5000 sampai 6000, 6000 sampai 7000): tekanan turbo dalam bar, pengapian dalam derajat, lambda.
- Bahan bakar (RON 95, 98, 100, atau 102), suhu luar, tekanan udara, dan transmisi (otomatis 8HP45 atau manual enam percepatan).
- **Standar** mengembalikan selisih ke nol, **Atur ulang** juga kondisinya.

Halaman menampilkan torsi dan tenaga standar dibanding yang diubah, setiap batas yang tersentuh beserta alasannya, dan nilai internal: tekanan turbo, udara pengisian, pengapian, cadangan knocking, suhu gas buang, turbo, pompa tekanan tinggi, tekanan efektif rata-rata.

## Cara model menghitung

1. **Udara.** Target tekanan turbo dibatasi oleh apa yang bisa dibangun turbo pada putaran itu, oleh rasio tekanannya, dan oleh alirannya. Kompresor memanaskan udara, intercooler mengambil sebagian panasnya lagi. Tekanan, suhu, dan efisiensi volumetrik memberi massa udara.
2. **Bahan bakar dan pengapian.** Bahan bakar mengikuti udara dan lambda. Pengapian adalah waktu standar ditambah selisih Anda, tetapi tidak pernah lebih maju daripada yang diizinkan batas knocking: kendali knocking mesin akan menariknya kembali. Batas itu bergeser dengan oktan, suhu udara pengisian, tekanan turbo, dan lambda.
3. **Keluaran.** Energi bahan bakar kali efisiensi yang turun makin jauh dari waktu pengapian terbaik, dikurangi gesekan, memberi torsi dan tenaga.
4. **Batas.** Turbo, suhu gas buang, pompa tekanan tinggi, injektor, pra-penyalaan pada putaran rendah, transmisi, udara pengisian, dan campuran masing-masing memberi peringatan dengan kata-kata sederhana.

Satu kurva utuh (25 titik dari 1000 sampai 7000 rpm) memakan sekitar 0,015 ms pada versi rilis.

## Data yang dipublikasikan

| Nilai | Angka | Sumber |
|---|---|---|
| Kapasitas, diameter, langkah, kompresi | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Torsi | 400 Nm dari 1200 sampai 5000 rpm | Wikipedia "BMW N55", data BMW untuk 435i |
| Tenaga | 225 kW (306 PS) pada 5700 sampai 6000 rpm | Wikipedia (5700 sampai 5800), BMW (5800 sampai 6000) |
| Putaran maksimum | 7000 rpm | Wikipedia "BMW N55" |
| Torsi masukan nominal 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Asumsi

Tidak ada yang memublikasikan peta internal N55 dengan lisensi bebas. Nilai-nilai ini asumsi saya; masing-masing dijelaskan di [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- **Tekanan turbo standar** dihitung mundur dengan model dari kurva yang dipublikasikan (sekitar 0,7 sampai 0,8 bar hingga 5000 rpm, di atasnya lebih rendah). Jadi cocoknya kurva standar menunjukkan konsistensi, bukan kebenaran.
- Efisiensi volumetrik 0,86 sampai 0,97, efisiensi indikasi 0,39, gesekan kuadratik terhadap putaran.
- Pengapian yang dibatasi knocking 3 sampai 15 derajat sebelum titik mati atas dengan cadangan 1,5 derajat; bergeser 1 derajat per angka oktan, 0,1 derajat per kelvin udara pengisian, 6 derajat per bar, dan 2,5 derajat per 0,1 lambda.
- Turbo: rasio tekanan hingga 2,55, aliran terkoreksi hingga 0,32 kg/s, efisiensi 0,72. Efektivitas intercooler 0,85, lebih rendah pada aliran tinggi.
- Batas suhu gas buang 980 °C, pompa tekanan tinggi 0,027 kg/s, jendela injeksi 40 % dari siklus, risiko pra-penyalaan di atas 18 bar tekanan efektif rata-rata di bawah 2500 rpm, manual enam percepatan sekitar 450 Nm.
- Bahan bakar E5: rasio udara bahan bakar 14,4, 42,4 MJ/kg.

## Seberapa dekat?

- Standar dibanding kurva yang dipublikasikan: 400 Nm dalam 1 % dari 1250 sampai 5000 rpm, 306 PS pada 6000 rpm, dan 304 PS pada 5750 rpm. Pengujian mengizinkan 3 %.
- Model belum diuji pada N55 sungguhan. Setiap angka adalah perkiraan: sekitar ±8 % untuk torsi dan tenaga, ±0,1 bar tekanan turbo, ±10 K udara pengisian, ±3 derajat cadangan knocking, ±60 K suhu gas buang.

## Membandingkan dengan mobil Anda

1. Sambungkan, buka **Nilai langsung** dengan putaran, beban, tekanan intake, suhu udara masuk, waktu pengapian, lambda, dan massa udara, lalu mulai merekam.
2. Berakselerasi penuh sekali melewati seluruh rentang putaran, hanya di tempat yang diizinkan dan aman.
3. Buka rekaman di halaman simulator. Caruspex mengambil titik beban penuh, mengelompokkannya per 500 rpm, dan menampilkan terukur dibanding simulasi: tekanan intake dan udara pengisian dengan peta Anda, massa udara dan pengapian dengan tekanan dan suhu terukur.

OBD hanya melaporkan tekanan intake sampai 255 kPa, dan sensor mana yang dipakai unit kendali untuk suhu udara masuk tidak didokumentasikan. **Lihat contoh** mengisi tabel dengan nilai rekaan agar Anda tahu cara membacanya.

## Baik untuk diketahui

- Torsi lebih besar dari standar pada putaran rendah menaikkan risiko pra-penyalaan, yang bisa mematahkan piston sekaligus.
- Pengapian lebih maju dari batas knocking tidak memberi apa-apa: kendali knocking menariknya kembali.
- Bahan bakar yang lebih baik saja memberi cadangan, bukan tenaga: peta pengapian standar adalah batas atasnya.
