# İlk adımlar

## 1. Demo aracı deneyin

Etrafa bakmak için araca ihtiyacınız yok. **Bağlan** sayfasında **Demo araç** seçin: sürer, ısınır, kayıtlı birkaç arızası vardır ve gerçek bir ELM327 gibi yanıt verir. Her sayfa onunla çalışır.

## 2. Dilinizi ve modunuzu seçin

Caruspex sisteminizin dilini kullanır. **Ayarlar** içinde 18 dilden birini ve açık ya da koyu görünümü seçebilirsiniz; ikisi de bir sonraki açılışta değişir.

**Başlangıç modu** günlük kullanım sayfalarını gösterir. **Profesyonel mod** servis araçlarını ekler: KKL sayfaları, adaptasyon, ham adaptör komutları ve kontrol ünitesi adresleri.

## 3. Aracınızı bağlayın

1. Teşhis soketini bulun: [bağlantı yardımcısı](connect.md) aracınızda nerede olduğunu gösterir.
2. Adaptörü takın ve kontağı açın. Motor çalışabilir ama çalışması gerekmez.
3. **Bağlan** sayfasında bağlantı türünü seçin ya da Caruspex'in tüm portları kendisi aramasına izin verin.

İlk açılışta bağlantı yardımcısı kendiliğinden açılır. Her adım atlanabilir ve yardımcıya daha sonra ayarlardan ve Bağlan sayfasından ulaşılabilir.

![Bağlan sayfası](screenshot-connect.png)

## 4. Araca ilk bakış

Bağlandıktan sonra Caruspex VIN'i okur ve bunun hangi araç olduğunu söyler. Başlamak için iyi yerler:

- **Sağlık kontrolü**: tek düğme, yarım dakika, kaybedilen her puanın açıklandığı 0 ile 100 arası bir puan.
- **Şimdi ne yapmalıyım?**: bulunan her şey, üç adıma ayrılmış halde.
- [Arıza belleği](faults.md) ve [canlı değerler](live.md).

![Sağlık kontrolü](screenshot-health.png)

## 5. Garaj ve raporlar

Her araç VIN'inden tanınır ve garajda kontrollerinin ve kilometre okumalarının geçmişiyle birlikte kendi kaydını alır. Kontrol raporu servis için PDF olarak kaydedilebilir.
