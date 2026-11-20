# Tuning simülatörü

**Tuning simülatörü** sayfası (profesyonel mod, Ölç altında) sanal bir BMW N55'tir: F30 335i ile F32/F36 435i'deki tek twin-scroll turbolu 3,0 litrelik sıralı altı silindir, 306 PS ve 400 Nm. Devir aralıklarına göre turbo basıncını, ateşlemeyi ve lambdayı değiştirin ve bir araca herhangi bir şey dokunmadan çok önce modelin ne dediğini görün.

> **Yalnızca simülasyon.** Caruspex motor haritası yazmaz ve asla yazılım yüklemez. Hiçbir flash dosya biçimini, sağlama toplamını ya da harita adresini bilmez ve hiçbir ticari tuning'i kopyalamaz. Gerçek tuning yerleşik araçlarla ve bir atölyede yapılır, değişikliğin onaylanması gerekir (Almanya'da TÜV tarafından). Onay olmadan araç tip onayını kaybeder.

![Tuning simülatörü](screenshot-tune.png)

## Neleri değiştirebilirsiniz

- Beş devir aralığında farklar (1000 ile 2000, 2000 ile 3500, 3500 ile 5000, 5000 ile 6000, 6000 ile 7000 arası): bar olarak turbo basıncı, derece olarak ateşleme, lambda.
- Yakıt (RON 95, 98, 100 ya da 102), dış sıcaklık, hava basıncı ve şanzıman (otomatik 8HP45 ya da altı ileri manuel).
- **Seri** farkları sıfırlar, **Sıfırla** koşulları da.

Sayfa seri ve değiştirilmiş tork ile gücü yan yana, değişikliğin dokunduğu her sınırı nedeniyle birlikte ve iç değerleri gösterir: turbo basıncı, şarj havası, ateşleme, vuruntu payı, egzoz sıcaklığı, turbo, yüksek basınç pompası, ortalama efektif basınç.

## Model nasıl hesaplar

1. **Hava.** Turbo basıncı hedefi, turbonun o devirde üretebildiğiyle, basınç oranıyla ve debisiyle sınırlanır. Kompresör havayı ısıtır, ara soğutucu ısının bir kısmını geri alır. Basınç, sıcaklık ve volümetrik verim hava kütlesini verir.
2. **Yakıt ve ateşleme.** Yakıt, hava ve lambdadan çıkar. Ateşleme seri avans artı sizin farkınızdır, ama vuruntu sınırının izin verdiğinden asla daha erken değildir: motorun vuruntu kontrolü onu geri alırdı. Sınır oktan, hava sıcaklığı, turbo basıncı ve lambda ile kayar.
3. **Güç.** Yakıt enerjisi çarpı en iyi avanstan uzaklaştıkça düşen bir verim, eksi sürtünme, tork ve gücü verir.
4. **Sınırlar.** Turbo, egzoz sıcaklığı, yüksek basınç pompası, enjektörler, düşük devir ön ateşleme, şanzıman, şarj havası ve karışım her biri sade sözlerle bir uyarı verir.

Bütün bir eğri (1000 ile 7000 dev/dk arasında 25 nokta) yayın sürümünde yaklaşık 0,015 ms sürer.

## Yayımlanmış veriler

| Değer | Rakam | Kaynak |
|---|---|---|
| Silindir hacmi, çap, strok, sıkıştırma | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Tork | 1200 ile 5000 dev/dk arası 400 Nm | Wikipedia "BMW N55", 435i için BMW verileri |
| Güç | 5700 ile 6000 dev/dk arası 225 kW (306 PS) | Wikipedia (5700 ile 5800), BMW (5800 ile 6000) |
| Azami devir | 7000 dev/dk | Wikipedia "BMW N55" |
| 8HP45'in anma giriş torku | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Varsayımlar

N55'in iç haritalarını kimse özgür lisansla yayımlamıyor. Bu değerler benim varsayımlarımdır; her biri [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs) içinde gerekçelendirilmiştir.

- **Seri turbo basıncı** yayımlanmış eğriden modelle geriye doğru hesaplanmıştır (5000 dev/dk'ya kadar yaklaşık 0,7 ile 0,8 bar, üstünde daha az). Seri eğrinin tutması bu yüzden doğruluğu değil, tutarlılığı gösterir.
- Volümetrik verim 0,86 ile 0,97, indike verim 0,39, sürtünme devre göre karesel.
- Vuruntuyla sınırlı ateşleme üst ölü noktadan önce 3 ile 15 derece, 1,5 derece pay ile; oktan başına 1 derece, hava için kelvin başına 0,1 derece, bar başına 6 derece ve 0,1 lambda başına 2,5 derece kayar.
- Turbo: basınç oranı 2,55'e kadar, düzeltilmiş debi 0,32 kg/s'ye kadar, verim 0,72. Ara soğutucu etkinliği 0,85, yüksek debide daha az.
- Egzoz sıcaklığı sınırı 980 °C, yüksek basınç pompası 0,027 kg/s, enjeksiyon penceresi çevrimin %40'ı, 2500 dev/dk altında 18 bar ortalama efektif basıncın üzerinde ön ateşleme riski, manuel şanzıman yaklaşık 450 Nm.
- E5 yakıt: hava yakıt oranı 14,4, 42,4 MJ/kg.

## Ne kadar doğru?

- Seri, yayımlanmış eğriye karşı: 1250 ile 5000 dev/dk arasında %1 içinde 400 Nm, 6000 dev/dk'da 306 PS ve 5750 dev/dk'da 304 PS. Testler %3'e izin verir.
- Model henüz gerçek bir N55'te denenmedi. Her rakam bir tahmindir: tork ve güçte yaklaşık ±%8, turbo basıncında ±0,1 bar, şarj havasında ±10 K, vuruntu payında ±3 derece, egzoz sıcaklığında ±60 K.

## Kendi aracınızla karşılaştırma

1. Bağlanın, devir, yük, manifold basıncı, emme havası sıcaklığı, ateşleme avansı, lambda ve hava kütlesiyle **Canlı değerler**'i açın ve bir kayıt başlatın.
2. Yalnızca izin verilen ve güvenli bir yerde bir kez tam gazla bütün devir aralığı boyunca hızlanın.
3. Kaydı simülatör sayfasında açın. Caruspex tam yük noktalarını alır, 500 dev/dk'lık aralıklarda gruplar ve ölçüleni simüle edilene karşı gösterir: manifold basıncı ve şarj havası sizin haritalarınızla, hava kütlesi ve ateşleme ölçülen basınç ve sıcaklıkla.

OBD manifold basıncını yalnızca 255 kPa'ya kadar bildirir ve motor beyninin emme sıcaklığı için hangi sensörü kullandığı belgelenmemiştir. **Örneği göster** tabloyu uydurma değerlerle doldurur, böylece nasıl okunduğunu görürsünüz.

## Bilmekte fayda var

- Düşük devirde seriden fazla tork, pistonları tek seferde kırabilecek ön ateşleme riskini artırır.
- Vuruntu sınırından daha erken ateşleme hiçbir şey kazandırmaz: vuruntu kontrolü onu geri alır.
- Daha iyi yakıt tek başına güç değil pay getirir: seri ateşleme haritası tavandır.
