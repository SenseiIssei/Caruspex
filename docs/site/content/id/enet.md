# Kabel ENET BMW

BMW dan Mini mulai seri F (sekitar 2008 ke atas) punya Ethernet di soket OBD. Kabel ENET berjalan dari soket OBD ke port jaringan komputer, tanpa chip adaptor di antaranya. Caruspex berbicara HSFZ dengan gateway mobil (port 6811 dan 6801) dan menjangkau setiap unit kontrol.

## Menyambungkan

1. Colokkan kabel ke mobil dan ke port jaringan komputer (atau adaptor jaringan USB). Nyalakan kontak.
2. Tunggu sampai satu menit: tanpa server DHCP, Windows, macOS, dan Linux memberi komputer alamat otomatis (169.254.x.x) sendiri.
3. Di halaman **Sambungkan** pilih **Kabel jaringan** dan biarkan kolomnya kosong: Caruspex mencari mobil di jaringan. Kalau pencarian tidak menemukan apa-apa, ketikkan alamat mobilnya.

## Yang bisa

- Setiap unit kontrol lewat UDS dengan pengalamatan BMW, kode kesalahan dalam format BMW seperti yang ditampilkan ISTA (misalnya 480A10).
- Memori kesalahan, identifikasi, dan penghapusan per unit kontrol, nilai langsung lewat OBD.
- Di sini pun Caruspex hanya membaca. Coding dan flashing tetap urusan program bengkel.

## Kabel yang mana?

Kabel ENET biasa punya resistor antara pin 8 dan 16 yang menyalakan Ethernet mobil. Kabel tanpa resistor itu tidak akan direspons mobil. Port jaringan 1 Gbit/s bisa dipakai, mobilnya berbicara pada 100 Mbit/s.

## Baris perintah

```
caruspex_cli ecus --enet auto
```
