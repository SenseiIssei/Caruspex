# Bağlantı yardımcısı ve OBD soketi

## Soket nerede?

Avrupa'da 2001'den (benzinli) veya 2004'ten (dizel), ABD'de ise 1996'dan beri satılan her aracın, sürücünün uzanabileceği yerde 16 pinli bir OBD soketi vardır. Alışılmış yerler:

- torpido altında, direksiyon kolonunun solunda veya sağında,
- pedalların üstünde,
- ayak boşluğunun yan kaplamasında,
- sigorta kutusunun kapağının arkasında,
- orta konsolda, bazen küçük bir kapağın arkasında.

Caruspex, 200'den fazla araç kuşağı ve 35 motosiklet için tam yeri bilir. Bağlantı yardımcısı bu yeri bir yerleşim planında ve sürücü tarafının yakın görünümünde işaretler; motosikletler için teşhis soketini ve gereken adaptör kablosunu içeren bir yan görünüm gösterir.

> Soket yerleri listesi kendi derlememizdir ve her araçta kontrol edilmemiştir. Yer sizin aracınız için yanlışsa lütfen bir [test raporu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) ile bize bildirin.

## Bağlantı yardımcısı

Yardımcı ilk açılışta kendiliğinden açılır. Dört şey sorar ve her adım atlanabilir:

1. **Hangi adaptör?** USB, Bluetooth veya Wi-Fi ile ELM327, bir ENET kablosu veya bir KKL kablosu. Resimler bunları ayırt etmeye yardımcı olur.
2. **Hangi araç?** Katalogdan seçin, ardından yardımcı soketin nerede olduğunu gösterir.
3. **Kontak açık.** Çoğu kontrol ünitesi yalnızca kontak açıkken yanıt verir.
4. **Bağlan.** Caruspex portları arar ve protokolleri dener.

Yardımcıyı daha sonra ayarlarda ve Bağlan sayfasında bulabilirsiniz.

## Arama ve hatırlama

Otomatik aramada Caruspex tüm seri portları dolaşır, her birinde bir ELM327 olup olmadığını sorar ve ardından aracın protokollerini tek tek dener. Bir araç bir kez bağlandığında Caruspex yolu (adaptör, port, protokol, bit hızı) o aracın VIN'i için hatırlar ve bir dahaki sefere önce onu dener; bu da bağlantıyı hızlandırır.

## Bağlanmazsa

Hiçbir adaptör yanıt vermezse veya araç sessiz kalırsa Caruspex, o bağlantı türü için kısa bir kontrol listesini, zaten tespit ettikleriyle birlikte gösterir: sürücü kurulu mu, Bluetooth eşleştirilmiş mi, kontak açık mı, bit hızı doğru mu, yalnızca daha yeni bir sürümmüş gibi davranan bir klon adaptör mü. [Adaptörler ve arayüzler](interfaces.md) altındaki sayfalar her türü ayrıntılı olarak açıklar.
