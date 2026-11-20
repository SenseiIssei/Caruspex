# Kabel K+DCAN

Kabel K+DCAN adalah kabel yang umum dipakai untuk BMW dari sekitar 1998 sampai 2017 dengan program bengkel INPA dan ISTA. Di dalamnya ada chip serial FTDI dan konverter kecil untuk D-CAN BMW. Banyak kabel seperti ini punya sakelar atau jumper antara pin 7 dan 8 untuk mobil yang lebih lama.

## Kondisi di Caruspex

> Keadaan sebenarnya: Caruspex membaca BMW seri E lewat K-Line kabel ini: DS2 pada model lama, KWP2000 gaya BMW dan BMW-FAST pada generasi E60. Sejauh ini baru diuji dengan mobil simulasi di dalam program, belum pada BMW sungguhan. Mode D-CAN kabel ini belum didukung Caruspex.

## Dalam mode K-Line

- **Setel sakelar ke K-Line** sebelum kabel masuk ke soket; pada kebanyakan kabel itu posisi kiri. Posisi ini menyambungkan pin 7 dan 8, sehingga unit kontrol di pin 8 pada model lama juga menjawab.
- **Mobil mana saja**: E36 (dengan colokan bundar 20 pin lewat adaptor), E38, E39, E46, E52, E53, E83, E85, dan generasi E60 (E60, E61, E63, E64, E65, E87, E90) hingga 03/2007. Setelah itu, dan mulai seri F, soketnya memakai D-CAN atau Ethernet.
- **Caranya**: nyalakan mode Pro, buka halaman *BMW lewat K-Line*, pilih port, *Pindai*. Caruspex menanyai setiap alamat yang dikenal lewat DS2 (9600 baud, paritas genap), KWP2000 (10400 baud dengan inisialisasi cepat), dan BMW-FAST (115200 baud), lalu menampilkan unit kontrol yang ditemukan beserta identitas dan memori kerusakannya. Butuh sekitar setengah menit.
- **Menghapus** bisa per unit kontrol atau di semua unit, keduanya hanya setelah ditanya. Sesudahnya Caruspex membaca ulang apa yang tersisa.
- **Kode kerusakan**: BMW tidak menerbitkan teks untuk kodenya, jadi Caruspex menampilkannya sebagai angka heksadesimal.
- **Hanya membaca**: tidak ada pengodean, adaptasi, atau uji aktuator di sini; itu tugas perangkat lunak bengkel.
- **Kabel KKL** bekerja dengan cara yang sama, tetapi pada model lama hanya menjangkau unit kontrol di pin 7 selama pin 7 dan 8 tidak tersambung.

Untuk mencoba tanpa mobil, daftar port di halaman ini berisi E46 (DS2) dan E60 (KWP2000) simulasi.

## Apa lagi yang bisa

- **BMW dan Mini mulai seri F (sekitar 2008 ke atas)**: [kabel ENET](enet.md) menjangkau setiap unit kontrol lewat Ethernet, atau [ELM327](elm327-usb.md) biasa dengan pengalamatan BMW sendiri lewat CAN.
- **OBD standar di BMW dengan CAN (sekitar 2007 ke atas)**: ELM327 apa pun bisa membaca kesalahan mesin dan nilai langsung.

## Butuh bantuan

Jika Anda punya kabel K+DCAN dan BMW lama, [laporan uji](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) beserta lognya membantu memastikan bagian K-Line di mobil sungguhan.
