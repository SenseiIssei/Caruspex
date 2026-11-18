# DoIP (IP üzerinden teşhis)

Yaklaşık 2016'dan sonraki birçok araç, DoIP adı verilen ISO 13400'e göre Ethernet üzerinden teşhis edilebilir. Bir DoIP kablosu OBD soketini bilgisayarın ağ portuna bağlar.

## Bağlanma

1. Kabloyu takın, kontağı açın ve bilgisayarın otomatik adresini (169.254.x.x) almasını bekleyin; bu bir dakikaya kadar sürebilir.
2. **Bağlan** sayfasında **Ağ kablosu** seçin ve alanı boş bırakın. Caruspex ağda DoIP araçlarını arar ve aracın DoIP mi yoksa BMW'nin HSFZ'sini mi konuştuğunu kendisi tanır.

## Neler çalışır

DoIP üzerinden OBD ve Caruspex'in o marka için adreslerini bildiği, UDS üzerinden kontrol üniteleri. DoIP araçları için adres listeleri henüz kısadır; gerçek araçlardan alınan kayıtlar bunları doldurur (paylaşılan bir kaydın neler içerdiği için bkz. [gizlilik](privacy.md)).

## Hangi kablo?

DoIP, OBD soketinin 3, 11, 12 ve 13 numaralı pinlerinde Ethernet hatlarına ve 8 numaralı pinde bir etkinleştirme hattına ihtiyaç duyar. Belirli bir marka için satılan kablolar genellikle tam olarak böyle bağlanmıştır; BMW ENET kablosu da bunlardan biridir.

## Komut satırı

```
caruspex_cli enet
```

ağda bulunan BMW ve DoIP araçlarını listeler.
