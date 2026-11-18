# DoIP (Diagnostics over IP)

Banyak mobil sejak sekitar 2016 bisa didiagnosis lewat Ethernet menurut ISO 13400, yang disebut DoIP. Kabel DoIP menghubungkan soket OBD dengan port jaringan komputer.

## Menyambungkan

1. Colokkan kabel, nyalakan kontak, dan tunggu sampai komputer mendapat alamat otomatis (169.254.x.x), yang bisa makan waktu sampai satu menit.
2. Di halaman **Sambungkan** pilih **Kabel jaringan** dan biarkan kolomnya kosong. Caruspex mencari kendaraan DoIP di jaringan dan mengenali sendiri apakah mobil memakai DoIP atau HSFZ milik BMW.

## Yang bisa

OBD lewat DoIP, dan unit kontrol lewat UDS yang alamatnya sudah diketahui Caruspex untuk merek itu. Daftar alamat untuk mobil DoIP masih pendek; rekaman dari mobil sungguhan akan melengkapinya (lihat [privasi](privacy.md) untuk isi rekaman yang dibagikan).

## Kabel yang mana?

DoIP butuh jalur Ethernet di pin 3, 11, 12, dan 13 soket OBD serta jalur aktivasi di pin 8. Kabel yang dijual untuk merek tertentu biasanya dirangkai persis seperti itu; kabel ENET BMW salah satunya.

## Baris perintah

```
caruspex_cli enet
```

menampilkan kendaraan BMW dan DoIP yang ditemukan di jaringan.
