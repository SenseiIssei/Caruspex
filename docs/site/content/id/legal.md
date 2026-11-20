# Keselamatan, merek dagang, dan lisensi

## Keselamatan dan tanggung jawab

Caruspex adalah proyek hobi yang bebas dan diberikan tanpa jaminan apa pun, seperti tertulis dalam [lisensi MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Anda menggunakannya dengan risiko sendiri.

- Jangan pernah mengoperasikannya saat mengemudi. Atur saat mobil berhenti; nilai langsung bisa dipantau oleh penumpang.
- Menghapus kesalahan, uji aktuator, dan pengaturan dasar membuat mobil melakukan sesuatu. Caruspex bertanya sebelum masing-masing; lakukan hanya saat mobil berhenti dan diamankan, dan hanya pada kendaraan yang boleh Anda kerjakan.
- Caruspex bukan pengganti bengkel. Kode kesalahan menunjukkan gejala, tidak selalu penyebabnya. Rem, airbag, kemudi, dan semua yang penting bagi keselamatan harus ditangani ahlinya.
- Menghapus bukan memperbaiki. Kesalahan yang dihapus dan monitor kesiapan yang direset bisa membuat uji emisi atau uji kelaikan kendaraan gagal.
- Caruspex tidak mengubah jarak tempuh, imobilizer, kunci, kendali emisi, atau tachograph, dan proyek ini tidak menerima kontribusi yang melakukannya. Memanipulasi odometer, kendali emisi, atau tachograph dilarang di banyak negara.

## Merek dagang

Nama pabrikan kendaraan, model, adaptor, program, dan standar (misalnya Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS, atau ISTA) adalah merek dagang pemiliknya masing-masing. Nama-nama itu hanya disebut untuk menyatakan mobil, adaptor, dan antarmuka apa saja yang bisa dipakai bersama Caruspex. Caruspex adalah proyek independen dan tidak berafiliasi dengan, tidak didukung oleh, dan tidak disponsori oleh pabrikan kendaraan, pembuat adaptor, atau vendor perangkat lunak mana pun. Aplikasi dan repositori ini tidak memakai logo perusahaan-perusahaan tersebut.

## Lisensi dan sumber

Kode berlisensi MIT, lihat [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Pustaka di dalam program dan lisensinya tercantum di [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); aplikasi menampilkannya di pengaturan. Data yang tertanam di dalam program:

- Nilai pabrikan untuk sekitar 550 kendaraan, dikonversi dari [OBDb](https://github.com/OBDb), berlisensi CC BY-SA 4.0 seperti OBDb: terpisah dari kode di [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) bersama lisensi dan daftar sumbernya ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Definisi sinyal CAN dari [opendbc](https://github.com/commaai/opendbc) (comma.ai) dan [model3dbc](https://github.com/joshwardell/model3dbc), lisensi MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Nilai baterai untuk Hyundai Ioniq dan Kona, Kia e-Niro, Nissan Leaf, dan Renault Zoe menurut OBDb (CC BY-SA 4.0, seperti paket ini), [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3), dan [evDash](https://github.com/nickn17/evDash) (keduanya MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Kode mesin dari artikel Wikipedia, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), dengan artikel dan revisi di setiap baris).
- Kode pabrikan VIN diperiksa dengan [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (domain publik) dan [Wikidata](https://www.wikidata.org) (CC0); kode yang hanya ada di [daftar WMI di Wikibuku](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) berlisensi CC BY-SA 4.0 dan disimpan terpisah di [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- Dimensi kendaraan dari [Wikidata](https://www.wikidata.org), CC0.
- Fon berlisensi SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
