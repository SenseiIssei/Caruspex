# Nilai langsung

Halaman **Nilai langsung** (mode profesional) menampilkan apa yang diukur sensor saat ini. Dalam mode pemula, **Mode berkendara** menampilkan nilai-nilai terpenting dengan huruf besar.

![Nilai langsung](screenshot-live.png)

## Apa saja yang ada

- Setiap nilai mode 01 dari SAE J1979 sampai PID C4: 145 PID dengan lebih dari 300 pembacaan, dikelompokkan dan bisa dicari. Mana yang tersedia di mobilmu, mobil itu sendiri yang memberi tahu.
- Odometer, gigi, suhu oli, konsumsi bahan bakar, torsi, tekanan turbo, suhu gas buang, filter partikel, AdBlue, NOx, dan baterai hybrid, sejauh mobil melaporkannya.
- Grafik berjalan untuk beberapa nilai sekaligus. Pada rentang waktu panjang (sampai satu jam) grafik menipiskan titik-titik lama tapi tetap menyimpan setiap puncak dan lembah.
- Merekam ke CSV.

## Nilai pabrikan dan nilai sendiri

- **Baterai mobil listrik**: kondisi kesehatan, isi daya, tegangan sel dengan selisihnya, dan suhu untuk Hyundai Kona dan Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf, serta Tesla Model 3 dan Y (dengan mendengarkan bus CAN), di halaman **Baterai EV**.
- **Nilai OBDb** untuk sekitar 550 kendaraan lagi (tekanan ban, oli, odometer, dan lainnya), ditawarkan sendiri untuk mobil yang terdeteksi.
- **Sesi diagnostik diperluas**: beberapa nilai mesin diesel grup Volkswagen, misalnya massa jelaga terukur dan jarak sejak regenerasi terakhir, hanya diberikan unit kontrol mesin dalam sesi diagnostik diperluasnya. Dalam mode profesional Caruspex memindahkan mesin ke sesi itu selama nilai tersebut ditanyakan, hanya untuk membaca, menandainya dengan **Sesi 03**, lalu mengembalikannya. Ini dapat dimatikan di pengaturan pada **Nilai pabrikan**. Jika unit kontrol menolak, misalnya saat berkendara, nilai tersebut menampilkan alasannya.
- **Nilai sendiri** dalam format CSV Torque yang banyak dipakai, termasuk rumusnya.

![Baterai EV](screenshot-battery.png)

## Seberapa cepat?

Berapa banyak nilai yang datang per detik tergantung mobil dan adaptornya. Mobil CAN menjawab cepat; mobil dengan K-Line memang lambat dari sananya. Dengan OBDLink, Caruspex bisa menanyakan sampai 24 nilai sekaligus (lihat [OBDLink dan STN](obdlink-stn.md)); antarmuka CAN mentah adalah cara yang paling cepat.

## Merekam dan memutar ulang

Satu sesi utuh dengan mobil bisa direkam ke file dan diputar ulang nanti tanpa mobil, praktis untuk mencari penyebab masalah. Dengan **Bagikan rekaman**, sebuah rekaman bisa disiapkan untuk proyek tanpa data pribadi, lihat [privasi](privacy.md).
