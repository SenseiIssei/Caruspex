# Güvenlik ve güncellemeler

Bu sayfa Caruspex'in güvenlik bilgilerini toplar: programın ne için olduğu, hangi risklerin bilinmesi gerektiği,
güncellemelerin nasıl çalıştığı ve programın nasıl kaldırılacağı. AB Siber Dayanıklılık Yasası'nın (Cyber
Resilience Act, Ek II) kullanıcılar için istediği bilgileri, özgür ve ticari olmayan bir programa uyduğu ölçüde izler.

## Program

- **Ad**: Caruspex, `caruspex` uygulaması ve `caruspex_cli` komut satırı aracı; Windows, Linux ve macOS için. Sürüm
  her indirmenin adında ve her sürümün değişiklik listesinde yazar.
- **Yayımlayan**: [GitHub](https://github.com/SenseiIssei/Caruspex) üzerindeki Caruspex projesi, MIT lisansıyla özgür
  yazılım olarak. Sorular ve hatalar [issues](https://github.com/SenseiIssei/Caruspex/issues) bölümüne.
- **Amaç**: otomobillerin, motosikletlerin ve kamyonların arıza teşhis verilerini kendi adaptörünüzle, kendi
  bilgisayarınızda okumak ve açıklamak. Hesap yok, bulut yok, izleme yok ([Gizlilik](privacy.md)).
- **Güvenlik özellikleri**: okumak araçta hiçbir şeyi değiştirmez; aracın bir şey yapmasına yol açan her işlem önce
  sorar. Bir güncelleme yalnızca imzası uygulamaya gömülü anahtarla eşleşirse kurulur. Caruspex ağ portu açmaz,
  dosyalardan ya da internetten gelen kodu çalıştırmaz.

## Güvenlik açığı bildirmek {#report}

Güvenlik sorunlarını herkese açık bir issue yerine
[yeni güvenlik bildirimi formuyla](https://github.com/SenseiIssei/Caruspex/security/advisories/new) gizli olarak
bildirin. [Güvenlik politikası](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) sonrasında ne olacağını
anlatır. Düzeltilen açıklar [güvenlik bildirimleri](https://github.com/SenseiIssei/Caruspex/security/advisories)
olarak yayımlanır.

## Bilinmesi gereken riskler {#risks}

- Caruspex'i yalnızca [sürümler sayfasından](https://github.com/SenseiIssei/Caruspex/releases) indirin. Başka
  yerlerden gelen kopyalar değiştirilmiş olabilir.
- Kayıtları, PID paketlerini ve 3D modelleri yalnızca güvendiğiniz kişilerden açın. Kod çalıştıramazlar, ama bozuk
  bir dosya Caruspex'i yavaşlatabilir ya da kapatabilir.
- Arıza kodlarını silmeyi, aktüatör testlerini ve temel ayarları yalnızca araç dururken ve güvenli bir yerde yapın.
- Birçok Bluetooth ve Wi-Fi adaptörünün PIN'i yoktur ya da ağı açıktır: yakındaki herkes bağlanabilir. İşiniz
  bitince adaptörü çıkarın.
- ENET kabloları, DoIP ve Wi-Fi adaptörleri hiçbir şeyi şifrelemez: onları doğrudan ya da yalnızca kendi ağınızda
  kullanın.
- J2534 sürücüleri üreticilerinin programlarıdır ve sizin yetkilerinizle çalışır. Onları yalnızca üreticiden kurun.

## Güncellemeler ve destek {#support}

- Caruspex açılışta GitHub'a daha yeni bir sürüm olup olmadığını sorar ve gösterir. Kurulum tek tıktır ve uygulama
  yalnızca imzası eşleşen paketi kurar. AppImage ve macOS uygulaması kendini günceller; .deb paketi ve Android
  uygulaması indirme sayfasını alır.
- Arama ve otomatik kurulum, ayarlarda **Pencere ve dil** altındaki anahtarlardır. Arama kapalıysa arada bir sürümler
  sayfasına bakın.
- Güvenlik düzeltmeleri yalnızca en yeni sürüme girer. Eski sürümler ayrı düzeltme almaz, bu yüzden Caruspex'i
  güncel tutun.
- Caruspex boş zamanlarda geliştirilir ve sabit bir bitiş tarihi yoktur. Bakım biterse depo ve son bir sürüm bunu
  önceden duyurur.

## İndirilen dosyayı doğrulamak {#verify}

Her sürümde her dosyanın sağlama toplamını içeren bir `SHA256SUMS` dosyası ve dosyaların nasıl derlendiğine dair
GitHub tarafından imzalanmış bir kayıt bulunur. Dosyalar aynı klasördeyken:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Bu araçlar yoksa PowerShell'de `Get-FileHash <dosya>` (Windows) ya da `shasum -a 256 <dosya>` (macOS), `SHA256SUMS`
içindeki satırla karşılaştırılacak toplamı gösterir.

## Caruspex'i ve verilerinizi kaldırmak {#remove}

1. Programı kaldırın: Windows'ta Ayarlar, Uygulamalar altından; macOS'te Caruspex'i Uygulamalar klasöründen
   sürükleyerek; Linux'ta AppImage dosyasını silerek ya da `sudo apt remove caruspex` ile.
2. Verilerinizi silin: ayarlardaki **Depolama konumunu aç** düğmesi ayarların, garajın ve çökme raporlarının
   bulunduğu klasörleri gösterir. Onları silin. Kendi kaydettiğiniz kayıtlar ve raporlar, kaydettiğiniz yerdedir.

## Geliştiriciler ve şirketler için {#integrators}

- Her sürümde her platform için bir yazılım malzeme listesi `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX) bulunur; her kitaplık sürümüyle birlikte listelenir.
- Risk değerlendirmesi, gereksinim listesi ve Cyber Resilience Act'in Caruspex için kapsamı
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra) içindedir.
- Caruspex, Cyber Resilience Act anlamında bir ürün olarak piyasaya arz edilmez; bu yüzden AB uygunluk beyanı ve CE
  işareti yoktur.
