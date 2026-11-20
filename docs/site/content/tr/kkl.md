# KKL kablosu (VAG 409.1)

KKL kablosu, içinde ELM327 olmayan, seri çipli ve K-Line sürücülü bir USB kablosudur. Caruspex K-Line'ı kendisi, bayt bayt sürer. Eski VW, Audi, Seat ve Skoda'ların ihtiyacı budur: protokolleri KWP1281 her bir baytı tek tek onaylar ve bunu hiçbir ELM327 yapamaz.

## Neler çalışır

- Yaklaşık 1990 ile 2004 arası VW, Audi, Seat ve Skoda'ların her kontrol ünitesi: motor, şanzıman, ABS, hava yastığı, gösterge paneli, konfor ve daha fazlası.
- KWP1281 ve KWP2000 (ISO 14230), uyandırmadan sonraki anahtar baytlardan otomatik olarak tanınır.
- Her kontrol ünitesi için arıza belleği, ölçüm blokları ve gaz kelebeği gövdesinin gerçek temel ayarı (ölçüm bloğu 060 veya 098).
- Adaptasyon kanalları bilerek yalnızca okunur. Çıkış testi gerçek parçaları çalıştırır ve yalnızca onayınızdan sonra başlar.

Bu sayfalar **profesyonel modda** bulunur: **KKL ile kontrol üniteleri**, **Gaz kelebeği gövdesi** ve **Adaptasyon ve çıkış testleri**. Her biri kabloyu kendisi açar: seri portu, kontrol ünitesini seçin, başlatın.

## Hangi kablo?

Orijinal FTDI FT232RL veya CP2102 çipli kablolar K-Line'da en ucuz klonlardan çok daha güvenilirdir. KWP1281 için orijinal bir FTDI önerilir. Kablo K-Line'ı ham olarak geçirmelidir: kendi yazılımı olan "HEX-USB" arayüzleri çalışmaz.

Yaklaşık 1996 öncesi araçlarda çoğu zaman OBD soketi yoktur, bunun yerine motor bölmesinde veya torpido altında 2x2 bir soket bulunur. 2x2'den 16 pinli sokete bir adaptör ucuzdur.

## Kabloyu kontrol edin

Komut satırı aracı kabloyu araç olmadan test eder:

```
caruspex_cli kklcheck --port COM3
```

## Araçta ilk test

Profesyonel moddaki **Araçta ilk test** sayfası, yeni bir KKL kablosuyla ilk oturumda size adım adım yol gösterir. Audi A2'ye göre hazırlanmıştır ve K-Line'lı her VW, Audi, Seat veya Skoda'da çalışır. Yalnızca okur: hiçbir şey silinmez, çıkış testi yok, temel ayar yok. Port ancak bir test başladığında açılır.

1. **Kabloyu araç olmadan kontrol edin.** Kabloyu yalnızca USB'ye takın, portunu seçin ve **Kabloyu kontrol et**'e basın. Caruspex çipi (FTDI, CH340, Prolific, CP210x), FTDI'da gecikme zamanlayıcısını okur ve break hattındaki 5 baud bir adresin bitlerini ölçer. Araç olmadan yankı gelmez, doğrusu da budur.
2. **Gecikme zamanlayıcısını 1 ms'ye ayarlayın** (Windows'ta FTDI kablolar): Aygıt Yöneticisi, Bağlantı Noktaları (COM ve LPT), USB Serial Port'a çift tıklayın, Bağlantı Noktası Ayarları, Gelişmiş, Latency Timer (msec) değerini 1 yapın, OK, ardından kabloyu bir kez çıkarıp yeniden takın. Varsayılan 16 ms ile anahtar baytlarına verilen yanıt 50 ms'lik penceresini kaçırabilir. Caruspex bu ayarı okur, ama asla değiştirmez.
3. **Araçta:** kontak açık, motor kapalı, kablo OBD soketine sıkıca ve doğrudan bilgisayara takılı (USB hub yok), başka bir teşhis programı açık değil. Listeyi işaretleyin ve **İlk testi başlat**'a basın.
4. Test motoru uyandırır (adres 01) ve adresin her bitini, 0x55 senkron baytını, anahtar baytlarını ve onlara verilen yanıtı ölçer. Ardından kimliği, arıza belleğini ve 001 ile 003 arası ölçüm bloklarını, sonra da gösterge paneli 17, ABS 03, hava yastığı 15, konfor 46 ve merkezi elektrik 09'u okur. Yanıt vermeyen bir kontrol ünitesi belki de araçta takılı değildir.
5. **Raporu kaydet**, Caruspex'in ayarlar klasöründeki `test-reports` klasörüne bir metin dosyası ve bir JSON dosyası yazar. **Test raporunu GitHub'da aç** rapor formunu sizin için doldurur. Kutuyu işaretlemezseniz VIN ve immobilizer numarası kısaltılır.

Sonuçların anlamı:

| Sonuç | Ne yapmalı |
|---|---|
| Yankı yok | Kablo kendini duymuyor: fiş tam oturmamış ya da 16 numaralı pinde 12 V yok (teşhis soketinin sigortasını kontrol edin). Araç olmadan bu normaldir. |
| Yankı bozuk | Sokette başka bir cihaz var, K+DCAN kablosunun anahtarı K-Line konumunda değil ya da kablo arızalı. |
| Uyandırma adresinin zamanlaması çok düzensiz | Bitler 200 ms'den 20 ms'den fazla sapıyor: USB hub kullanmayın, diğer programları kapatın, gecikme zamanlayıcısı 1 ms. Caruspex break çağrılarını öne alarak bir kez daha dener. |
| Kablo sağlam, ama araçtan yanıt yok | Kontak açık mı? Adres 01 motordur. Yaklaşık 2005'ten itibaren araçlar CAN üzerinden teşhis edilir. |
| Anahtar baytlarına yanıt çok geç geldi | Gecikme zamanlayıcısını 1 ms'ye ayarlayın. |

## Tipik sorunlar

- **Kontrol ünitesinden yanıt yok**: kontak açık mı? 5 baud uyandırma tam bir zamanlama ister; USB hub'ları ve bazı sanal makineler bunu bozar, kabloyu doğrudan takın.
- **Bir süre sonra kopuyor**: ucuz kablolar bayt kaybeder. Aracı suçlamadan önce başka bir kablo deneyin.
