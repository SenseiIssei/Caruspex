# Bluetooth üzerinden ELM327

Kablosuz küçük soketler. İki tür Bluetooth vardır ve Caruspex ikisini de konuşur.

## Bluetooth LE

Vgate iCar Pro BLE, vLinker, OBDLink CX ve Veepeak BLE gibi yeni adaptörler Bluetooth Low Energy kullanır. **Eşleştirme gerekmez**: **Bağlan** sayfasında **Bluetooth** seçin, **Bluetooth cihazlarını ara** düğmesine tıklayın ve listeden adaptörü seçin. Bu Windows, Linux ve macOS'ta çalışır.

## Klasik Bluetooth

Eski ve birçok ucuz adaptör, seri profilli klasik Bluetooth kullanır.

1. Adaptörü sisteminizin Bluetooth ayarlarında bir kez eşleştirin. PIN genellikle `1234` veya `0000`'dır.
2. Caruspex'te **Bluetooth** seçin ve eşleştirilmiş adaptörü seçin; liste bir cihazın LE mi, klasik mi yoksa ikisi birden mi olduğunu söyler.

Windows ve Linux'ta Caruspex eşleştirilmiş klasik adaptörlere doğrudan bağlanır. Alternatif olarak Windows adaptör için bir COM portu oluşturur; bu da [USB üzerinden ELM327](elm327-usb.md) gibi çalışır. macOS'ta klasik Bluetooth adaptörleri yalnızca sistem onlar için bir seri port sunarsa çalışır; orada Bluetooth LE'li adaptörler daha iyi bir seçimdir.

## İpuçları

- Kullanmadığınızda adaptörü çıkarın: birçok ucuz model aracın aküsünü meşgul eder.
- Adaptör görünmüyorsa bilgisayardaki Bluetooth'u kapatıp yeniden açın ve aynı anda ona bağlı bir telefon uygulaması olmadığından emin olun.
- Bluetooth kablodan daha yavaştır. Bir OBDLink ile Caruspex aynı anda birkaç değer sorar, bu da farkın çoğunu kapatır (bkz. [OBDLink ve STN](obdlink-stn.md)).
