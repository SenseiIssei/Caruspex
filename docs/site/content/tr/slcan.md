# SLCAN (Lawicel)

SLCAN, CAN çubukları için seri port üzerinden çalışan basit bir metin protokolüdür: slcan yazılımlı CANable, USBtin, Lawicel CANUSB ve birçok ev yapımı cihaz. Windows, Linux ve macOS'ta çalışır.

## Bağlanma

1. Çubuğu bilgisayara, bir kabloyla da OBD soketine bağlayın: CAN high pin 6'ya, CAN low pin 14'e, şase pin 4 ve 5'e.
2. **Bağlan** sayfasında **CAN arayüzü** seçin. SLCAN çubukları otomatik port aramasıyla bulunur ve listede görünür.
3. Bit hızını seçin (CAN üzerinden OBD için 500 kbit/s) ve bağlanın.

## Neler çalışır

Her ham CAN arayüzünde olduğu gibi: OBD, UDS üzerinden her kontrol ünitesi, kontrol ünitelerinin paralel aranması, VW TP 2.0 ve yayınlanan değerleri dinleme. SLCAN üzerinden CAN FD mümkün değildir.

## İpuçları

- candleLight yazılımlı bir CANable SLCAN cihazı değildir: Linux'ta bir [SocketCAN](socketcan.md) arayüzü olarak görünür.
- Çubuğun OBD soketinde sonlandırma direncine ihtiyacı yoktur, aracın kendi direnci vardır.

## Komut satırı

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
