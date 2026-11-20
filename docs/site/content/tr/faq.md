# Sık sorulan sorular

## Caruspex ücretli mi?

Hayır. Caruspex, MIT lisansı altında özgür bir yazılımdır; reklam, hesap veya ücretli ek yoktur.

## Hangi adaptörü almalıyım?

Çoğu kişi için en iyi başlangıç, FTDI veya CH340 çipli bir USB ELM327 ya da bir OBDLink'tir (Bluetooth veya USB). Yaklaşık 2004 öncesi VW, Audi, Seat ve Skoda için bir [KKL kablosu](kkl.md) ekleyin; F serisinden itibaren BMW ve Mini için bir [ENET kablosu](enet.md). [Arayüzlere genel bakış](interfaces.md) bunları karşılaştırır.

## Aracımla çalışır mı?

OBD-II veya EOBD'li her araç motor için çalışır: Avrupa'da 2001'den itibaren benzinliler ve 2004'ten itibaren dizeller, ABD'de 1996'dan itibaren. Yaklaşık 1990'dan sonraki eski VW grubu araçlar KKL kablosuyla çalışır. **Araçlar** sayfası belirli bir araç için hangi yolun işe yaradığını söyler. Euro 5'ten itibaren motosikletler, teşhis soketleri için bir adaptör kablosuyla çalışır.

## Benim aracımda test edildi mi?

Dürüst olmak gerekirse: henüz değil. Caruspex simüle edilmiş kontrol ünitelerine karşı test edilir; gerçek donanımda şimdiye kadar yalnızca STN2232 çipli bir OBDLink kontrol edildi, araç olmadan. Lütfen bir [test raporu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) ile yardım edin.

## Caruspex aracımı bozabilir mi?

Caruspex çoğunlukla okur. Yalnızca size sorduktan sonra yazar: arıza kodlarını silme ve VAG KKL kablosuyla aktüatör testi ile kelebek gövdesi temel ayarı. Asla kodlama yapmaz, asla yazılım yüklemez ve asla adaptasyon değerlerini değiştirmez. Sürüş sırasında kullanmayın.

## Kilometreyi, immobilizeri değiştirebilir veya özellik kodlayabilir mi?

Hayır, yapmayacak da. Bu hukuken hassas bir konudur ve bu aracın amacı değildir.

## Adaptör neden 2.1 sürümünü bildiriyor ama bazı şeyler başarısız oluyor?

Birçok ucuz adaptör, gerçekte olduğundan daha yeni bir ELM327 sürümü bildirir. **Adaptör testi**, adaptörün gerçekte hangi komutları bildiğini ortaya çıkarır.

## Eski aracımda değerler neden yavaş?

K-Line'daki araçlar (Avrupa'da yaklaşık 2001 ile 2007 arası) doğası gereği yavaş yanıt verir. CAN'li araçlar çok daha hızlıdır.

## Caruspex verilerimi bir yere gönderiyor mu?

Hayır. Yalnızca güncelleme kontrolü GitHub'a en yeni sürümü sorar, o da ancak siz evet dedikten sonra; bunu istediğiniz zaman değiştirebilirsiniz. Bkz. [gizlilik](privacy.md).

## Nasıl yardım edebilirim?

Aracınızdan bir test raporu gönderin, bir kayıt paylaşın, hataları bildirin ya da bir çeviriyi iyileştirin: rehber [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md) dosyasıdır ve ayarlardaki **Çeviriyi iyileştir** düğmesi onu açar.

Kod da memnuniyetle karşılanır: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md), Caruspex'in nasıl derleneceğini ve bir pull request için nelerin gerektiğini (İngilizce) anlatır.
