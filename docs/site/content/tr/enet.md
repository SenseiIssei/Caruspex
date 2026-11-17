# BMW ENET kablosu

F serisinden itibaren BMW ve Mini'lerde (yaklaşık 2008'den sonra) OBD soketinde Ethernet vardır. Bir ENET kablosu, arada adaptör çipi olmadan OBD soketinden bilgisayarın ağ portuna gider. Caruspex aracın ağ geçidiyle HSFZ konuşur (portlar 6811 ve 6801) ve her kontrol ünitesine ulaşır.

## Bağlanma

1. Kabloyu araca ve bilgisayarın ağ portuna (veya bir USB ağ adaptörüne) takın. Kontağı açın.
2. Bir dakikaya kadar bekleyin: DHCP sunucusu olmadığında Windows, macOS ve Linux bilgisayara kendiliğinden otomatik bir adres (169.254.x.x) verir.
3. **Bağlan** sayfasında **Ağ kablosu** seçin ve alanı boş bırakın: Caruspex aracı ağda arar. Arama bir şey bulamazsa aracın adresini yazın.

## Neler çalışır

- BMW adreslemesiyle UDS üzerinden her kontrol ünitesi, arıza kodları ISTA'nın gösterdiği BMW biçiminde (örneğin 480A10).
- Her kontrol ünitesi için arıza belleği, tanımlama ve silme, OBD üzerinden canlı değerler.
- Caruspex burada da yalnızca okur. Kodlama ve yazılım yükleme servis programlarına kalır.

## Hangi kablo?

Sıradan bir ENET kablosunda pin 8 ile 16 arasında aracın Ethernet'ini açan bir direnç vardır. Bu direnç olmayan kablolar ölü kalır. 1 Gbit/s ağ portları çalışır, araç 100 Mbit/s konuşur.

## Komut satırı

```
caruspex_cli ecus --enet auto
```
