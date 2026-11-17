# USB üzerinden ELM327

OBD soketine takılan ve USB portuna giden kablolu bir soket. ELM327'nin en güvenilir türüdür: eşleştirme yok, radyo yok, biten pil yok.

## Sürücü

Soketin içinde bir USB çipi kabloyu bir seri porta (Windows'ta COM portu) dönüştürür. Hangi sürücünün gerektiği bu çipe bağlıdır:

| Çip | Sürücü |
|---|---|
| FTDI FT232 | genellikle Windows kendisi kurar, Linux ve macOS'ta yerleşik |
| CH340 / CH341 | genellikle Windows kendisi kurar, Linux ve macOS'ta yerleşik |
| Silicon Labs CP210x | Windows bulamazsa üreticinin web sitesinden |
| Prolific PL2303 | üreticinin web sitesinden; eski klonlar çoğu zaman yalnızca eski bir sürücüyle çalışır |

Taktıktan sonra Windows Aygıt Yöneticisi adaptörü "Bağlantı noktaları (COM ve LPT)" altında gösterir.

## Bağlanma

1. Adaptörü araca takın, kontağı açın, kabloyu bilgisayara takın.
2. **Bağlan** sayfasında **USB veya COM portu** seçin.
3. Portu **Otomatik ara** konumunda bırakın ya da listeden seçin, ardından bağlanın.

Otomatik arama her seri portu alışılmış hızlarda dener. Adaptörünüzün hızını biliyorsanız ayarlarda **Adaptör baud hızı** altında belirleyebilirsiniz.

## Linux ve macOS

Linux'ta adaptör `/dev/ttyUSB0` veya `/dev/ttyACM0` olarak görünür. Kullanıcınızın `dialout` grubunda olması gerekir (bkz. [Kurulum](install.md)). macOS'ta `/dev/cu.usbserial-...` veya `/dev/cu.wchusbserial...` olarak görünür.

## Tipik sorunlar

- **Hiç port görünmüyor**: sürücü eksik ya da kablo yalnızca şarj kablosu.
- **Adaptör yanıt veriyor, araç vermiyor**: kontak kapalı ya da adaptör aracın protokolünü kaldıramayan bir klon. [Adaptör testi](interfaces.md) size daha fazlasını söyler.
- **Eski araçlarda çok yavaş canlı değerler**: K-Line'ın kendisi yavaştır, bu adaptörün suçu değildir.
