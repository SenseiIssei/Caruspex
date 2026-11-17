# K+DCAN kablosu

K+DCAN kablosu, yaklaşık 1998 ile 2017 arası BMW'ler için INPA ve ISTA servis programlarıyla kullanılan alışılmış kablodur. İçinde bir FTDI seri çipi ve BMW'nin D-CAN'i için küçük bir dönüştürücü vardır. Bu kabloların çoğunda eski araçlar için bir anahtar veya pin 7 ile 8 arasında bir köprü bulunur.

## Caruspex'teki durum

> Dürüst durum: Caruspex bu kablonun D-CAN modunu henüz konuşmuyor. K-Line üzerinden eski BMW protokolleri (DS2 ve BMW'ye özgü biçimdeki KWP2000) planlanıyor ama önce gerçek bir araçta test edilmeleri gerekiyor.

Bugün bunun yerine kullanabilecekleriniz:

- **F serisinden itibaren BMW ve Mini (yaklaşık 2008'den sonra)**: bir [ENET kablosu](enet.md) Ethernet üzerinden her kontrol ünitesine ulaşır, ya da CAN üzerinden BMW'nin kendi adreslemesiyle sıradan bir [ELM327](elm327-usb.md).
- **CAN'li BMW'lerde standart OBD (yaklaşık 2007'den sonra)**: her ELM327 motor arızalarını ve canlı değerleri okur.
- **K-Line modunda** kablo, K-Line'ı bir [KKL kablosu](kkl.md) gibi geçirir. Bu VW grubu araçlar için kullanışlıdır; K-Line üzerindeki BMW kontrol ünitelerinin henüz eksik olan protokollere ihtiyacı vardır.

## Yardım aranıyor

Bir K+DCAN kablonuz ve eski bir BMW'niz varsa, kayıt içeren bir [test raporu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) bu bölümün geliştirilmesine yardımcı olur.
