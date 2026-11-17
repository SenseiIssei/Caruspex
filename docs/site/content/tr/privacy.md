# Gizlilik

Caruspex bilgisayarınızda çalışır. Hesabı, bulutu ve izlemesi yoktur.

## Neler kaydedilir

Her şey bilgisayarınızda kalır:

| Ne | Ne zaman | Nerede |
|---|---|---|
| Ayarlar | her zaman | sisteminizin ayarlar klasörü (ayarlardaki **Depolama konumunu aç** düğmesi bunu gösterir) |
| Garaj: VIN, araç, kontroller, kilometre okumaları, son çalışan bağlantı | bir araç bağlandıktan sonra | ayarların yanında, Caruspex veri klasöründe |
| Kayıtlar ve CSV dosyaları | yalnızca siz başlattığınızda | onları kaydettiğiniz yerde |
| Günlük dosyası | yalnızca ayarlarda açarsanız | seçtiğiniz yol |
| Çökme raporları | yalnızca Caruspex çökerse | garajın yanındaki `crash-reports` klasörü |

Bu dosyalardan herhangi birini istediğiniz zaman silebilirsiniz.

## Neler gönderilir

Caruspex açılışta GitHub'a daha yeni bir sürüm olup olmadığını sorar. Bu sırada sizinle veya aracınızla ilgili hiçbir şey gönderilmez. Kontrol ayarlardan kapatılabilir. Bunun dışında Caruspex internete bağlanmaz. Bir Wi-Fi adaptörü, ENET kablosu veya DoIP araçla iletişim yerel ağınızda kalır.

## Çökme raporları yalnızca istek üzerine

Caruspex çökerse bilgisayarınıza kısa bir rapor yazar: sürüm, sistem, koddaki yer ve çağrı zinciri. VIN'ler, araç trafiğinin baytları ve ana klasörünüz çıkarılır. Hiçbir şey gönderilmez. Caruspex size raporu gösterir ve iki düğme sunar: **GitHub'da bildir**, tarayıcınızda önceden doldurulmuş bir kayıt (issue) açar, siz onu yayımlamadan önce okuyup değiştirirsiniz; **Bildirme** ise raporu kenara kaldırır.

## Kayıtları kişisel veri olmadan paylaşma

Gerçek araçlardan alınan kayıtlar Caruspex'i daha fazla araç için daha iyi yapmaya yardımcı olur. **Kaydı paylaş** bir kopya hazırlar ve neyin kalıp neyin gittiğini size tam olarak gösterir:

- **Kaldırılanlar**: VIN'in son haneleri (dosyanın her yerinde), port adı ve adaptörün Bluetooth veya ağ adresi, günün saati.
- **Bir kayıtta asla bulunmayanlar**: konumunuz ve kullanıcı verileriniz.
- **Kalanlar**: istekler ve yanıtlar, adaptör türü, protokol ve tarih.

Dosya, siz kaydedip kendiniz bir GitHub kaydına (issue) ekleyene kadar bilgisayarınızda kalır. Hiçbir şey otomatik olarak yüklenmez.

## Test programı

[Test raporu formu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) aracı, yılı, motoru ve adaptörü sorar. Yalnızca kısaltılmış bir VIN ister; tam VIN'i, plakanızı veya adınızı asla paylaşmayın.
