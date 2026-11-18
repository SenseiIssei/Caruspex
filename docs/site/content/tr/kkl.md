# KKL kablosu (VAG 409.1)

KKL kablosu, içinde ELM327 olmayan, seri çipli ve K-Line sürücülü bir USB kablosudur. Caruspex K-Line'ı kendisi, bayt bayt sürer. Eski VW, Audi, Seat ve Skoda'ların ihtiyacı budur: protokolleri KWP1281 her bir baytı tek tek onaylar ve bunu hiçbir ELM327 yapamaz.

## Neler çalışır

- Yaklaşık 1990 ile 2004 arası VW, Audi, Seat ve Skoda'ların her kontrol ünitesi: motor, şanzıman, ABS, hava yastığı, gösterge paneli, konfor ve daha fazlası.
- KWP1281 ve KWP2000 (ISO 14230), uyandırmadan sonraki anahtar baytlardan otomatik olarak tanınır.
- Her kontrol ünitesi için arıza belleği, ölçüm blokları ve gaz kelebeği gövdesinin gerçek temel ayarı (ölçüm bloğu 060 veya 098).
- Adaptasyon kanalları ve çıkış testleri bilerek yalnızca okunur.

Bu sayfalar **profesyonel modda** bulunur: **KKL ile kontrol üniteleri**, **Gaz kelebeği gövdesi** ve **Adaptasyon ve çıkış testleri**. Her biri kabloyu kendisi açar: seri portu, kontrol ünitesini seçin, başlatın.

## Hangi kablo?

Orijinal FTDI FT232RL veya CP2102 çipli kablolar K-Line'da en ucuz klonlardan çok daha güvenilirdir. KWP1281 için orijinal bir FTDI önerilir. Kablo K-Line'ı ham olarak geçirmelidir: kendi yazılımı olan "HEX-USB" arayüzleri çalışmaz.

Yaklaşık 1996 öncesi araçlarda çoğu zaman OBD soketi yoktur, bunun yerine motor bölmesinde veya torpido altında 2x2 bir soket bulunur. 2x2'den 16 pinli sokete bir adaptör ucuzdur.

## Kabloyu kontrol edin

Komut satırı aracı kabloyu araç olmadan test eder:

```
caruspex_cli kklcheck --port COM3
```

## Tipik sorunlar

- **Kontrol ünitesinden yanıt yok**: kontak açık mı? 5 baud uyandırma tam bir zamanlama ister; USB hub'ları ve bazı sanal makineler bunu bozar, kabloyu doğrudan takın.
- **Bir süre sonra kopuyor**: ucuz kablolar bayt kaybeder. Aracı suçlamadan önce başka bir kablo deneyin.
