# Tampilan tembus

**Tampilan tembus** menampilkan mobil yang tersambung sebagai model kaca yang bisa kamu putar. Di dalamnya kamu melihat mesin atau motor listrik, transmisi, tangki atau baterai tegangan tinggi, knalpot, radiator, aki 12 V, dan soket OBD, ditata sesuai jenis bodi dan penggerak mobil yang terdeteksi.

![Tampilan tembus](screenshot-xray.png)

## Apa yang menyala

- **Kesalahan** mewarnai areanya, dan unit kontrol yang bertanggung jawab berubah merah.
- **Nilai langsung** menempel sebagai label di komponen asalnya: suhu cairan pendingin di radiator, putaran mesin di mesin, tegangan di aki.
- **Unit kontrol** yang ditemukan pencarian berada di tempat biasanya, dihubungkan oleh jalur datanya (CAN powertrain, CAN kenyamanan, CAN diagnosis, K-Line). Selama Caruspex membaca, paket-paket kecil bergerak ke unit yang sedang ditanyai.
- **Periksa mobil** menerbangkan kamera ke setiap temuan, satu per satu.

Putar dengan mouse atau jari, zoom dengan roda mouse atau dua jari; keyboard juga bisa. Lapisan (bodi, penggerak, pendinginan, bahan bakar atau tegangan tinggi, kelistrikan, jaringan) bisa dimatikan satu per satu.

## Seberapa akurat modelnya?

Bentuknya diambil dari dimensi asli generasi itu kalau diketahui (dari Wikidata atau data teknis pabrikan), dan selain itu dari nilai yang umum untuk jenis bodi dan zamannya. Ini bukan replika model yang persis: data desain dan diagram kelistrikan pabrikan tidak tersedia bebas. Halaman ini menyebutkan dari mana bentuknya berasal.

Kalau kamu punya model 3D mobilmu dalam glTF (`.glb` atau `.gltf`), kamu bisa memuatnya dengan **Muat model 3D milikmu**; bagian dalamnya tetap dari tata letak.

## Tanpa 3D

Di komputer tanpa WebGL, halaman ini beralih ke tampak samping datar dengan informasi yang sama.
