# Pertanyaan yang sering diajukan

## Apakah Caruspex berbayar?

Tidak. Caruspex adalah perangkat lunak bebas dengan lisensi MIT, tanpa iklan, tanpa akun, dan tanpa fitur tambahan berbayar.

## Adaptor mana yang sebaiknya saya beli?

Untuk kebanyakan orang, ELM327 USB dengan chip FTDI atau CH340, atau OBDLink (Bluetooth atau USB), adalah awal yang paling baik. Untuk VW, Audi, Seat, dan Skoda sebelum sekitar 2004 tambahkan [kabel KKL](kkl.md); untuk BMW dan Mini mulai seri F, [kabel ENET](enet.md). [Ikhtisar antarmuka](interfaces.md) membandingkan semuanya.

## Apakah bisa dipakai di mobil saya?

Mobil apa pun dengan OBD-II atau EOBD bisa untuk mesinnya: di Eropa mobil bensin sejak 2001 dan diesel sejak 2004, di AS sejak 1996. Mobil lama grup VW sejak sekitar 1990 bisa lewat kabel KKL. Halaman **Kendaraan** memberi tahu cara mana yang cocok untuk mobil tertentu. Sepeda motor mulai Euro 5 bisa dengan kabel adaptor untuk colokan diagnosisnya.

## Apakah sudah diuji di mobil saya?

Jujur saja: belum. Caruspex diuji dengan unit kontrol simulasi; di perangkat keras sungguhan sejauh ini baru satu OBDLink dengan chip STN2232 yang dicek, tanpa mobil. Tolong bantu dengan [laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Apakah Caruspex bisa merusak mobil saya?

Caruspex hanya membaca. Satu-satunya yang ia tulis adalah penghapusan kode kesalahan, dan ia memperingatkan sebelumnya. Ia tidak pernah melakukan coding, tidak pernah flashing, dan tidak pernah mengubah nilai adaptasi. Jangan memakainya sambil menyetir.

## Bisakah ia mengubah jarak tempuh, immobilizer, atau coding fitur?

Tidak, dan tidak akan. Itu sensitif secara hukum dan bukan tujuan alat ini.

## Kenapa adaptor mengaku versi 2.1 tapi ada yang tidak jalan?

Banyak adaptor murah melaporkan versi ELM327 yang lebih baru daripada aslinya. **Tes adaptor** mencari tahu perintah mana yang benar-benar dikenal adaptor.

## Kenapa nilai di mobil lama saya lambat?

Mobil dengan K-Line (di Eropa sekitar 2001 sampai 2007) memang menjawab lambat dari sananya. Mobil CAN jauh lebih cepat.

## Apakah Caruspex mengirim data saya ke mana-mana?

Tidak. Hanya pemeriksaan pembaruan yang menanyakan versi terbaru ke GitHub, dan itu bisa dimatikan. Lihat [privasi](privacy.md).

## Bagaimana saya bisa membantu?

Kirim laporan uji dari mobilmu, bagikan rekaman, laporkan bug, atau perbaiki terjemahan: panduannya ada di [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), dan tombol **Perbaiki terjemahan** di pengaturan membukanya.

Kode juga sangat diterima: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) menjelaskan (dalam bahasa Inggris) cara membangun Caruspex dan apa yang dibutuhkan sebuah pull request.
