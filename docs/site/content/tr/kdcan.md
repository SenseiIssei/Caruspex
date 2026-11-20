# K+DCAN kablosu

K+DCAN kablosu, yaklaşık 1998 ile 2017 arası BMW'ler için INPA ve ISTA servis programlarıyla kullanılan alışılmış kablodur. İçinde bir FTDI seri çipi ve BMW'nin D-CAN'i için küçük bir dönüştürücü vardır. Bu kabloların çoğunda eski araçlar için bir anahtar veya pin 7 ile 8 arasında bir köprü bulunur.

## Caruspex'teki durum

> Dürüst durum: Caruspex, BMW E serisini bu kablonun K-Line'ı üzerinden okur: eski modellerde DS2, E60 kuşağında BMW usulü KWP2000 ve BMW-FAST. Şimdiye kadar yalnızca programdaki simülasyon araçlarla denendi, henüz gerçek bir BMW'de değil. Kablonun D-CAN modunu Caruspex henüz konuşmuyor.

## K-Line modunda

- Kablo sokete takılmadan önce **anahtarı K-Line'a** alın; çoğu kabloda bu sol konumdur. 7 ve 8 numaralı pinleri birleştirir, böylece eski modellerde 8. pindeki kontrol üniteleri de yanıt verir.
- **Hangi araçlar**: E36 (20 pinli yuvarlak fişle, bir adaptör üzerinden), E38, E39, E46, E52, E53, E83, E85 ve 03/2007'ye kadar E60 kuşağı (E60, E61, E63, E64, E65, E87, E90). Sonrasında ve F serisinden itibaren soket D-CAN veya Ethernet konuşur.
- **Nasıl**: profesyonel mod açık, *K-Line ile BMW* sayfası, portu seçin, *Tara*. Caruspex bilinen her adresi DS2 (9600 baud, çift eşlik), KWP2000 (hızlı başlatmalı 10400 baud) ve BMW-FAST (115200 baud) üzerinden sorar ve bulunan kontrol ünitelerini kimlik ve arıza belleğiyle gösterir. Yaklaşık yarım dakika sürer.
- **Silme** tek bir ünitede veya her yerde yapılabilir, ikisi de ancak sorulduktan sonra. Ardından Caruspex kalanları yeniden okur.
- **Arıza kodları**: BMW kodlarının metinlerini yayımlamaz, bu yüzden Caruspex onları onaltılık sayı olarak gösterir.
- **Yalnızca okuma**: burada kodlama, uyarlama veya aktüatör testi yok; bunlar servis yazılımının işidir.
- **Bir KKL kablosu** aynı şekilde çalışır, ancak eski modellerde 7 ve 8 numaralı pinler birleşik değilse yalnızca 7. pindeki ünitelere ulaşır.

Araç olmadan denemek için sayfanın port listesinde simülasyon bir E46 (DS2) ve E60 (KWP2000) bulunur.

## Başka neler çalışır

- **F serisinden itibaren BMW ve Mini (yaklaşık 2008'den sonra)**: bir [ENET kablosu](enet.md) Ethernet üzerinden her kontrol ünitesine ulaşır, ya da CAN üzerinden BMW'nin kendi adreslemesiyle sıradan bir [ELM327](elm327-usb.md).
- **CAN'li BMW'lerde standart OBD (yaklaşık 2007'den sonra)**: her ELM327 motor arızalarını ve canlı değerleri okur.

## Yardım aranıyor

Bir K+DCAN kablonuz ve eski bir BMW'niz varsa, kayıtlı bir [test raporu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) K-Line kısmını gerçek bir araçta doğrulamaya yardımcı olur.
