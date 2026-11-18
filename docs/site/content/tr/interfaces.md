# Adaptörler ve arayüzler

Caruspex neyin takılı olduğunu ve aracın hangi dili konuştuğunu kendisi bulur. Araca hangi yoldan girilirse girilsin aynı işlevler çalışır: OBD-II, UDS üzerinden her kontrol ünitesi, yayınlanan değerleri dinleme, kayıt ve yeniden oynatma.

| Arayüz | Örnekler | Sayfa |
|---|---|---|
| USB üzerinden ELM327 | ucuz ELM327 kabloları, Vgate, Veepeak | [USB üzerinden ELM327](elm327-usb.md) |
| Bluetooth üzerinden ELM327 | klasik Bluetooth ve Bluetooth LE soketler, vLinker, Vgate iCar Pro | [Bluetooth üzerinden ELM327](elm327-bluetooth.md) |
| Wi-Fi üzerinden ELM327 | kendi Wi-Fi ağını açan soketler | [Wi-Fi üzerinden ELM327](elm327-wifi.md) |
| STN çipleri | OBDLink SX, EX, MX+, CX | [OBDLink ve STN](obdlink-stn.md) |
| KKL kablosu (VAG 409.1) | FTDI veya CP2102 kablolar | [KKL kablosu](kkl.md) |
| K+DCAN kablosu | anahtarlı BMW kabloları | [K+DCAN kablosu](kdcan.md) |
| ENET kablosu | F serisinden itibaren BMW ve Mini | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, candleLight'lı CANable, Raspberry Pi CAN kartları | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP kablosu | yaklaşık 2016'dan itibaren birçok araç | [DoIP](doip.md) |

## Hangisi ne yapabilir?

Sıradan bir ELM327; motor, şanzıman ve yaklaşık 2008'den sonraki çoğu araçta UDS üzerinden diğer tüm kontrol üniteleri için yeterlidir. Her bir baytı tek tek onaylayan eski Volkswagen grubu protokolü KWP1281'i yapamaz: yaklaşık 2004 öncesi VW, Audi, Seat ve Skoda için bir KKL kablosu gerekir.

Ham CAN arayüzleri (J2534, SLCAN, SocketCAN, PCAN, Kvaser) Caruspex'e çıplak CAN çerçevelerini verir. Taşıma protokolünü Caruspex kendisi yapar; bu da kontrol ünitelerinin hızlı paralel aranmasını ve yaklaşık 2004 ile 2012 arası Volkswagen grubu araçlar için VW TP 2.0'ı mümkün kılar.

![Adaptör testi](screenshot-adapter.png)

## Adaptörüm orijinal mi?

**Adaptör testi**, adaptöre her ELM327 sürümünün komutlarını sorar, yanıt sürelerini ölçer ve bundan ne çıktığını dürüstçe söyler: birçok ucuz adaptör 1.5 veya 2.1 sürümünü bildirir ama yalnızca 1.0'ın komutlarını bilir. Caruspex adaptöre asla bir şey yazmaz.

> Şimdiye kadar test edilen: STN2232 çipli bir OBDLink adaptör testini geçti, araç olmadan. Diğer tüm adaptörler ve arayüzler belgelerine göre geliştirildi ama henüz gerçek bir araçta test edilmedi.
