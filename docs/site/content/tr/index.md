# Caruspex

Caruspex, otomobiller ve motosikletler için ücretsiz bir arıza teşhis programıdır. Windows, Linux ve macOS üzerinde çalışır, 18 dil konuşur ve insanların gerçekten sahip olduğu adaptörlerle çalışır: ucuz ELM327 soketi, bir OBDLink, eski Volkswagen grubu araçlar için bir KKL kablosu, BMW için bir ENET kablosu ve profesyonel CAN arayüzleri.

![Canlı göstergelerle genel bakış](screenshot-overview.png)

## Neler yapar

- Arıza kodlarını okur ve siler, bunları sade bir dille açıklar ve yola devam edip edemeyeceğinizi söyler.
- Canlı değerleri gösterir: sıcaklıklar, hız, karışım, turbo basıncı, akü voltajı ve çok daha fazlası, hem sayı hem grafik olarak.
- Aracın yalnızca motorunu değil, araç ve adaptör izin verdiği ölçüde her kontrol ünitesini sorgular.
- İkinci el bir aracı satın almadan önce kontrol eder: VIN tutuyor mu, kilometreler birbirini tutuyor mu, kısa süre önce arızalar silinmiş mi?
- Aracı, arızaların ve değerlerin ait oldukları yerde yandığı cam bir model olarak gösterir.
- Sonra ne yapmanız gerektiğini söyler: hemen durun, yakında servise gidin ya da bekleyebilir.

## Nereden başlamalı

1. Bilgisayarınıza [Caruspex'i kurun](install.md).
2. [İlk adımları](first-steps.md) izleyin: önce demo aracı deneyin, sonra kendi aracınızı bağlayın.
3. Adaptörünüzü [adaptörler ve arayüzler](interfaces.md) bölümünde bulun.
4. Caruspex'in neyi kaydettiğini ve gönderdiğini [gizlilik](privacy.md) bölümünde okuyun. Kısacası: siz kendiniz paylaşmadıkça bilgisayarınızdan hiçbir şey çıkmaz.

> Caruspex çoğunlukla okur. Yalnızca size sorduktan sonra yazar: arıza kodlarını silme ve VAG KKL kablosuyla aktüatör testi ile kelebek gövdesi temel ayarı. Kontrol ünitelerini asla kodlamaz, kilometreyi asla değiştirmez ve immobilizere asla dokunmaz.

## Dürüst durum

Caruspex, şartnamelere göre geliştirilmiş ve simüle edilmiş kontrol ünitelerine, sahte ağ geçitlerine ve sahte sürücülere karşı test edilmiştir. Gerçek donanımda şimdiye kadar yalnızca STN2232 çipli bir OBDLink kontrol edildi, araç olmadan. Caruspex'i aracınızda denerseniz lütfen bir [test raporu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) gönderin: her rapor test edilmiş araçlar listesini uzatır.
