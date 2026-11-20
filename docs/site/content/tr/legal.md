# Güvenlik, ticari markalar ve lisanslar

## Güvenlik ve sorumluluk

Caruspex özgür bir hobi projesidir ve [MIT lisansında](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) yazdığı gibi hiçbir garanti olmadan sunulur. Kullanım riski size aittir.

- Sürüş sırasında asla kullanmayın. Araç dururken ayarlayın; canlı değerleri bir yolcu izleyebilir.
- Arıza silme, aktüatör testleri ve temel ayarlar araca bir şey yaptırır. Caruspex her birinden önce sorar; bunları yalnızca araç durmuş ve emniyete alınmışken ve yalnızca üzerinde çalışma izniniz olan araçlarda yapın.
- Caruspex bir servisin yerini tutmaz. Bir arıza kodu bir belirtiyi gösterir, her zaman nedenini değil. Frenler, hava yastıkları, direksiyon ve güvenlik için önemli olan her şey uzman ellere bırakılmalıdır.
- Silmek onarmak değildir. Silinen arızalar ve sıfırlanan hazırlık testleri bir emisyon veya araç muayenesinin başarısız olmasına yol açabilir.
- Caruspex kilometreyi, immobilizeri, anahtarları, emisyon kontrolünü veya takografı değiştirmez ve proje bunu yapan katkıları kabul etmez. Kilometre sayaçlarına, emisyon kontrolüne veya takograflara müdahale birçok ülkede yasaktır.

## Ticari markalar

Araç üreticilerinin, modellerin, adaptörlerin, programların ve standartların adları (örneğin Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS veya ISTA) sahiplerinin ticari markalarıdır. Yalnızca Caruspex'in hangi araçlar, adaptörler ve arayüzlerle çalıştığını belirtmek için geçer. Caruspex bağımsız bir projedir; hiçbir araç üreticisi, adaptör üreticisi veya yazılım sağlayıcısıyla bağlantılı değildir, onlar tarafından onaylanmamış ve desteklenmemiştir. Uygulama ve bu depo bu şirketlerin logolarını kullanmaz.

## Lisans ve kaynaklar

Kod MIT lisanslıdır, bkz. [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Programlardaki kütüphaneler ve lisansları [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md) dosyasında listelenir; uygulama bunları ayarlarda gösterir. Programlara gömülü veriler:

- Yaklaşık 550 araç için üretici değerleri, [OBDb](https://github.com/OBDb) kaynağından dönüştürülmüş, OBDb gibi CC BY-SA 4.0 lisanslı: koddan ayrı olarak [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) klasöründe lisans ve kaynak listesiyle ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- [opendbc](https://github.com/commaai/opendbc) (comma.ai) ve [model3dbc](https://github.com/joshwardell/model3dbc) kaynaklı CAN sinyal tanımları, MIT lisansı ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Hyundai Ioniq ve Kona, Kia e-Niro, Nissan Leaf ve Renault Zoe için batarya değerleri OBDb (CC BY-SA 4.0, bu paketler gibi), [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) ve [evDash](https://github.com/nickn17/evDash) (ikisi de MIT) kaynaklıdır: [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Wikipedia makalelerinden motor kodları, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), her satırda makale ve sürümüyle).
- VIN üretici kodları [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (kamu malı) ve [Wikidata](https://www.wikidata.org) (CC0) ile doğrulandı; yalnızca [Wikibooks'taki WMI listesinde](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) yer alan kodlar CC BY-SA 4.0 lisanslıdır ve ayrı olarak [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi) içinde durur.
- [Wikidata](https://www.wikidata.org) kaynaklı araç ölçüleri, CC0.
- SIL Open Font License 1.1 lisanslı yazı tipleri: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
