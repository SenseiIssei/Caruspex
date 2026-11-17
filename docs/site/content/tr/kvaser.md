# Kvaser (Windows)

Kvaser'in CAN adaptörleri (Leaf, U100, Memorator ve diğerleri) Windows'ta Kvaser'in CANlib kitaplığı üzerinden doğrudan kullanılabilir. Linux'ta bunlar [SocketCAN](socketcan.md) arayüzleridir.

## Bağlanma

1. Kvaser sürücülerini kurun. Bunlar, Caruspex'in yüklediği `canlib32.dll` dosyasını getirir.
2. **Bağlan** sayfasında **CAN arayüzü** seçin. Arkasında cihaz olan her Kvaser kanalı listede görünür; sürücünün sanal kanalları dışarıda bırakılır.
3. Bit hızını seçin (CAN üzerinden OBD için 500 kbit/s) ve bağlanın.

## Neler çalışır

OBD, UDS üzerinden her kontrol ünitesi, kontrol ünitelerinin paralel aranması, VW TP 2.0 ve yayınlanan değerleri dinleme. CANlib üzerinden CAN FD henüz eklenmedi.

## Komut satırı

```
caruspex_cli read --can kvaser:0@500000
```
