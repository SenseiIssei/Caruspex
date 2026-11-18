# OBDLink ve STN çipleri

OBDLink adaptörleri (SX, EX, MX+, CX, LX), OBD Solutions firmasının STN çiplerini kullanır. Bir ELM327'nin anladığı her şeyi ve daha fazlasını anlarlar; Caruspex onları kendisi tanır ve ek özelliklerini kullanır.

## Caruspex'in kullandıkları

- **Toplu sorgular**: Caruspex tek seferde 6 yerine 24'e kadar canlı değer sorar. Bluetooth üzerinden bu, bir değer turunu neredeyse iki kat hızlandırır.
- **Dinleme**: yayınlanan değerler (örneğin bir Tesla'da) STN'nin kendi komutlarıyla kaydedilir; bu komutlar belirlenen sayıda mesajdan sonra kendiliğinden durur.
- **Ford MS-CAN ve GM Single-Wire-CAN**: Ford ve GM'nin gövde kontrol üniteleri normal teşhis CAN hattında bulunmaz. Ek alıcı-vericiye sahip bir adaptörle **Tüm kontrol üniteleri** sayfası "Ford: MS-CAN üzerindeki gövde üniteleri" veya "GM: Single-Wire-CAN üzerindeki gövde üniteleri" seçeneğini sunar.

| Çip | Adaptör | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 ile STN2232 arası | OBDLink EX | evet | hayır |
| STN1150 ile 1152 arası, 1170, 2120, 2255, 2256 | OBDLink MX+ ve diğerleri | evet | evet |
| diğerleri | OBDLink SX, LX, CX | hayır | hayır |

## Bağlanma

OBDLink adaptörleri her ELM327 gibi bağlanır: USB (EX ve SX), Bluetooth (MX+, LX, CX) üzerinden ya da [USB üzerinden ELM327](elm327-usb.md) ve [Bluetooth üzerinden ELM327](elm327-bluetooth.md) sayfalarında anlatıldığı gibi.

## Test edildi

STN2232 çipli bir OBDLink adaptör testini geçti: çip, yazılım sürümü ve komutlar doğru tanındı. Henüz bir araçta kullanılmadı; bu yüzden toplu sorgular, MS-CAN ve SW-CAN gerçek donanımda henüz doğrulanmadı.
