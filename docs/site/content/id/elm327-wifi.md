# ELM327 lewat Wi-Fi

Beberapa adaptor membuka jaringan Wi-Fi sendiri. Komputer bergabung ke jaringan itu dan berbicara dengan adaptor lewat TCP.

## Menyambungkan

1. Colokkan adaptor dan nyalakan kontak. Setelah beberapa detik muncul jaringan Wi-Fi, sering bernama `WiFi_OBDII`, `V-LINK`, atau nama pembuatnya.
2. Sambungkan komputermu ke jaringan itu. Jaringan ini tidak punya internet, jadi sistemmu mungkin memperingatkan bahwa koneksinya terbatas: itu tidak apa-apa.
3. Di Caruspex pilih **Wi-Fi** di halaman **Sambungkan**. Kebanyakan adaptor memakai alamat `192.168.0.10` dan port `35000`; ini sudah diatur. Kalau adaptormu memakai yang lain (misalnya `192.168.4.1`), ketikkan sebagai `address:port`.

## Tips

- Selama komputer ada di jaringan adaptor, ia tidak punya internet. Itu tidak masalah bagi Caruspex.
- Adaptor Wi-Fi berjalan di semua sistem tanpa driver.
- Kalau koneksinya putus, dekatkan komputer ke mobil: sinyal radio colokan seperti ini lemah.
