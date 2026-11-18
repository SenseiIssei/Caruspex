# PEAK PCAN (Windows)

PEAK-System'in CAN adaptörleri (PCAN-USB, PCAN-USB FD, PCAN-USB Pro ve diğerleri) Windows'ta J2534 sürücüsü olmadan doğrudan kullanılabilir. Linux'ta bunlar [SocketCAN](socketcan.md) arayüzleridir.

## Bağlanma

1. PEAK'in cihaz sürücüsünü kurun. Bu sürücü, Caruspex'in yüklediği `PCANBasic.dll` kitaplığını getirir.
2. **Bağlan** sayfasında **CAN arayüzü** seçin. Arkasında cihaz olan her PCAN kanalı listede kendiliğinden görünür; başka bir program tarafından zaten kullanılan kanallar dışarıda bırakılır.
3. Bit hızını seçin (CAN üzerinden OBD için 500 kbit/s) ve bağlanın.

## Neler çalışır

OBD, UDS üzerinden her kontrol ünitesi, kontrol ünitelerinin paralel aranması, VW TP 2.0 ve yayınlanan değerleri dinleme. PCAN-Basic üzerinden CAN FD henüz eklenmedi.

## Komut satırı

```
caruspex_cli read --can pcan:USB1@500000
```
