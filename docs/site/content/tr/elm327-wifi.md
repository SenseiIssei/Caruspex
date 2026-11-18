# Wi-Fi üzerinden ELM327

Bazı adaptörler kendi Wi-Fi ağlarını açar. Bilgisayar bu ağa katılır ve adaptörle TCP üzerinden konuşur.

## Bağlanma

1. Adaptörü takın ve kontağı açın. Birkaç saniye sonra bir Wi-Fi ağı görünür; adı çoğu zaman `WiFi_OBDII`, `V-LINK` ya da üreticinin adıdır.
2. Bilgisayarınızla bu ağa katılın. Ağın interneti yoktur, bu yüzden sisteminiz bağlantının sınırlı olduğu konusunda uyarabilir: bu normaldir.
3. Caruspex'te **Bağlan** sayfasında **Wi-Fi** seçin. Çoğu adaptör `192.168.0.10` adresini ve `35000` portunu kullanır; bu önceden ayarlıdır. Sizinki başka bir adres kullanıyorsa (örneğin `192.168.4.1`), bunu `address:port` biçiminde yazın.

## İpuçları

- Bilgisayar adaptörün ağındayken internete erişemez. Bu Caruspex için önemli değildir.
- Wi-Fi adaptörleri her sistemde sürücü olmadan çalışır.
- Bağlantı koparsa bilgisayarı araca yaklaştırın: bu soketlerin radyosu zayıftır.
