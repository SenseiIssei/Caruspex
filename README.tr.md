# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · **Türkçe** · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Windows, Linux ve macOS için açık kaynaklı araç teşhisi; Rust ile yazıldı, arayüzü Tauri 2 kullanıyor.
Tek bir kablo ya da adaptör, tek bir uygulama: arıza kodlarını okuyun ve silin, canlı sensör verilerini izleyin,
emisyon testine hazır olup olmadığını kontrol edin ve VW/Audi kontrol ünitelerine K-Line kablosuyla bayt düzeyinde bağlanın.

Ad, bağırsaklardan alametler okuyan Romalı kâhin *haruspex*'ten geliyor. Caruspex ise arabanınkini okur.

Uygulama 18 dilde konuşur; arayüz de her arıza kodu metni de bunlara dahildir: İngilizce, Almanca, İspanyolca,
Fransızca, Portekizce, İtalyanca, Felemenkçe, Lehçe, Çekçe, Rusça, Ukraynaca, Türkçe, Arapça (sağdan sola), Çince,
Japonca, Korece, Hintçe ve Endonezce. Sistem dilini izler, ayarlardan değiştirilebilir ya da
çalıştırma başına `--language fr` gibi bir seçenekle zorlanabilir. Çevirilerin nasıl yapıldığı ve denetlendiği
[translations/TRANSLATING.md](translations/TRANSLATING.md) dosyasında anlatılıyor.

![Canlı göstergelerle genel bakış](docs/screenshot-overview.png)

## Öne çıkanlar

- **İki eksiksiz teşhis yığını**: herhangi bir ELM327 adaptörüyle standart OBD-II/EOBD ve ham KKL K-Line
  kablosuyla VW/Audi üretici teşhisi (KWP1281 ve KWP2000).
- **Protokoller sıfırdan yazıldı**: KWP1281 her baytı tümleyeniyle onaylar,
  ELM327 yazılımı bunu yapamaz. Caruspex K-Line'ı kendisi sürer; seri portun break sinyalini
  süre sınırlarına dayalı zamanlamayla değiştirerek bit bit gönderilen 5 baud uyandırma adresi de buna dahildir, böylece
  işletim sistemi zamanlayıcısının dalgalanması on bit boyunca birikmez.
- **Otomatik protokol tanıma**: uyandırmadan sonra ünitenin iki anahtar baytı onun
  KWP1281 mi yoksa KWP2000 (ISO 14230) mü konuştuğunu gösterir; Caruspex buna göre geçiş yapar.
- **Modüler Rust çalışma alanı**: her alan için bir crate (adaptör bağlantıları, protokoller, ağ ve CAN, araçlar, veri,
  kayıtlar, analiz, demo araç, komut satırı aracı) ve en üstte Tauri uygulaması. Çekirdek arayüzden habersizdir,
  bu yüzden uygulama ve komut satırı aracı tamamen aynı kod üzerinde çalışır.
- **Araç olmadan test edildi**: simüle edilmiş ünitelerle (`crates/caruspex-protocols/tests/common/`)
  bayt bayt el sıkışma yeniden üretilir, sahte ağ geçitleri ENET ve DoIP'in yerini tutar, sahte bir J2534 sürücüsü PassThru yolunu kapsar
  ve Python ile yazılmış bir ELM327 simülatörü uygulamanın baştan sona çalıştırılmasına olanak tanır.
- **Tek bir çeviri takımı**: çekirdek ve arayüz `translations/<code>/` içindeki aynı çeviri dosyalarını
  paylaşır, 18 dilde. Çekirdek metinlerini İngilizce metne göre bulur, arayüz
  kataloglarını aynı dosyalardan oluşturur.
- **Her arabayı tanır**: 365 üretici kodlu bir VIN çözücü ve 1980'den beri 2.100'den fazla
  model neslinin kataloğu, hangi aracın bağlı olduğunu ve onunla neyin çalıştığını söyler.
- **Yerleşik demo araç**: CAN üzerinde motoru ve şanzımanı olan, aynı süreç içinde çalışan bir ELM327; böylece her şey
  donanım olmadan denenebilir.
- **Elle yazılmış değil, üretilmiş veri**: yaklaşık 1.700 arıza kodu, bunların çoğu düzenli kod ailelerinden
  (silindir başına, sıra başına, oksijen sensörü başına) `tools/gen_dtc.py` ile hesaplanıyor.

## Özellikler

### Arabayı tanır
- Bağlandıktan sonra Caruspex VIN'i okur ve hangi araba olduğunu söyler: marka, model, nesil, model yılı.
- 94 markadan 2.100'den fazla model neslinin kataloğu, 1980'den bugüne, artık var olmayan markalar dahil.
- Her nesil için: hangi teşhis soketi, nerede durduğu, hangi protokol ve OBD-II'nin, bir KKL kablosunun mu yoksa yalnızca üreticinin kendi araçlarının mı işe yarayacağı.
- Herhangi bir VIN için VIN denetleyicisi: üretici, ülke, model yılı, kontrol hanesi, model kodu.
- Her arabayı VIN'inden tanıyan ve kontrollerinin ile kilometre sayacı okumalarının geçmişini tutan bir garaj.

![Araçlar](docs/screenshot-vehicles.png)

### Tek düğmeyle sağlık kontrolü
- Arıza belleğini, hazır olma monitörlerini ve bileşen testlerini okur, ardından yarım dakika boyunca karışımı, sıcaklıkları, oksijen sensörlerini ve şarj voltajını izler.
- 0 ile 100 arasında bir puan; kaybedilen her puan sade sözlerle ve bir sonraki adımla açıklanır.
- Emisyon testine hazır mı? Hangi monitör hâlâ açık ve bitirmek için nasıl sürülmeli.
- Yazdırmak ya da servise vermek için PDF olarak bir muayene raporu.

![Sağlık kontrolü](docs/screenshot-health.png)

### Arıza belleği
- Kayıtlı, bekleyen ve kalıcı kodlar; birden çok kontrol ünitesi varsa her kod onu kimin bildirdiğini gösterir (motor, şanzıman).
- Yaklaşık 1.400 standart kod (P, C, B, U) ile VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru ve daha fazlası için üretici kodları. Bir P1 kodu, algılanan markanın anladığı şekilde okunur.
- Her kod için bir trafik ışığı ve herkesin ilk sorduğu soru: sürmeye devam edebilir miyim?
- Olağan nedenler, kendiniz kontrol edebileceğiniz şeyler ve kabaca bir maliyet sınıfı.
- Şeffaf görünümdeki araba, etkilenen parçanın nerede durduğunu gösterir.
- Net bir uyarıyla silme ve otomatik yeniden okuma; CSV dışa aktarma.

![Arıza belleği](docs/screenshot-faults.png)

### Şimdi ne yapmalıyım?
- Caruspex'in bulduğu her şeyi üç adıma ayıran tek bir sayfa: hemen durun, yakında servise, bekleyebilir.
- Her madde için nedenini anlatan düz bir cümle, kabaca bir maliyet aralığı ve serviste göstermek için bir not (kod, kontrol ünitesi, bölge, olağan nedenler).
- OBD, motor lambasının yanıp sönüp sönmediğini ya da kırmızı bir uyarı lambasının yanıp yanmadığını göremez: bunu iki işaret kutusu sorar, yanıp sönen lamba ve ateşleme kaçırma birlikteyse madde "Hemen durun" bölümüne taşınır.

![Şimdi ne yapmalıyım](docs/screenshot-todo.png)

### İkinci el aracı kontrol et
- Satın almadan önce aracı görmeye giderken: araçtaki şasi numarası (VIN) belgelerle ve katalogla uyuşuyor mu, kilometre kontrol üniteleri arasında tutarlı mı, arızalar kısa süre önce mi silinmiş (silmeden bu yana mesafe ve süre)?
- Hazırlık monitörleri, kayıtlı arızalar, sağlık kontrolü ve elektrikli araçlarda akü durumu.
- Bir trafik ışığı, satıcıya sorulacak sorular ve satın alma için bir PDF.

![İkinci el aracı kontrol et](docs/screenshot-used-car.png)

### 3D Röntgen görünümü
- Bağlı araba, çevirebileceğiniz cam bir model olarak: motor veya motorlar, şanzıman, depo veya yüksek gerilim aküsü, egzoz, radyatör, 12 V akü ve OBD soketi; algılanan arabanın kasa tipine ve aktarma sistemine göre yerleştirilir.
- Taramada bulunan her kontrol ünitesi genelde durduğu yerde durur ve veri hatlarıyla (güç aktarma ve konfor CAN) bağlanır; Caruspex okurken sorulan üniteye küçük paketler gider.
- Arızalar bölgesini yakar ve sorumlu kontrol ünitesini kırmızıya boyar, canlı değerler ait oldukları parçanın üzerinde durur ve "Aracı kontrol et" kamerayı her bulguya uçurur. Katmanlar tek tek kapatılabilir.
- Kasa tipi ve döneme göre tipik ölçüler, tam modelin kopyası değil: üreticilerin tasarım verileri ve elektrik şemaları serbestçe bulunmuyor.

![Röntgen görünümü](docs/screenshot-xray.png)

### Canlı değerler
- SAE J1979'un PID C4'e kadar her mod 01 değeri: 300'den fazla okumalı 145 PID, gruplanmış ve aranabilir.
- Kilometre sayacı, vites, yağ sıcaklığı, yakıt tüketimi, tork, turbo basıncı, egzoz sıcaklıkları, partikül filtresi, AdBlue, NOx ve hibrit akü, araba bildirdiği ölçüde.
- Kayan grafik ve CSV kaydı.

![Canlı değerler](docs/screenshot-live.png)

### Ölçüm
- Yolculuk bilgisayarı: canlı ve ortalama tüketim, maliyet, CO2, mesafe ve sakin sürüş tarzı puanı; kendi yolculuk bilgisayarı olmayan arabalar için de.
- Performans: hız sinyalinden 0'dan 100 km/s'ye, 80'den 120'ye, çeyrek mil ve fren mesafesi (yalnızca kapalı yollarda).
- Akü ve alternatör: boşta voltaj, marş çökmesi, yüklü ve yüksüz şarj voltajı, dört adımda yönlendirmeli.

### Muayene
- Hazır olma monitörleri sade sözlerle, dondurulmuş kare, VIN.
- Değer ve sınırlarla mod 06 bileşen izleme, silindir başına ateşleme kaçırma sayıları.
- Mod 09'dan ECU adları, kalibrasyon kimlikleri ve sağlama toplamları, kontrol ünitesi başına.
- Ham ELM327 terminali.

### Adaptör testi
- Orijinal ELM327 mi, klon mu? Her sürümün komutlarını sorar, yanıt süresini ölçer ve bundan ne çıktığını dürüstçe söyler.
- OBDLink ve diğer STN adaptörlerini tanır.
- Adaptöre asla bir şey yazmaz.

![Adaptör testi](docs/screenshot-adapter.png)

### Her kablo, arabaya giden her yol
Caruspex neyin takılı olduğunu ve arabanın ne konuştuğunu kendisi bulur.

| Arayüz | Örnekler | Nasıl |
|---|---|---|
| USB, Bluetooth, Bluetooth LE, Wi-Fi üzerinden ELM327 | ucuz ELM327, Vgate, vLinker, Veepeak | seri port, eşleştirilmiş ya da doğrudan BLE, TCP |
| STN yongaları | OBDLink SX, EX, MX+, CX | ELM327 gibi, daha hızlı zamanlamayla |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, J2534 kipinde VAS 5054A | üreticinin 64 bit sürücüsü, ISO-TP'li CAN'ı Caruspex yapar |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | seri port, otomatik port aramasıyla bulunur |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, candleLight'lı CANable, Raspberry Pi CAN hat kartları | can0 ve benzerleri |
| KKL kablosu (VAG 409.1) | FTDI veya CP2102 kablolar | K-Line, KWP1281 ve KWP2000 |
| ENET kablosu | F serisinden itibaren BMW ve Mini | Ethernet üzerinden HSFZ |
| DoIP kablosu | yaklaşık 2016'dan itibaren birçok araba | Ethernet üzerinden ISO 13400 |

Bunların hepsinde aynı işlevler çalışır: OBD-II ve UDS üzerinde OBD, UDS ile her kontrol ünitesi, BMW
adreslemesi, yayın değerlerini dinleme (Tesla), kayıt ve tekrar oynatma.

![Bağlan](docs/screenshot-connect.png)

### CAN üzerinden tüm kontrol üniteleri (UDS)
- Sıradan bir ELM327 ile Caruspex her kontrol ünitesine tek tek sorar: motor, şanzıman, ABS, hava yastığı, gösterge paneli, klima ve daha fazlası, yaklaşık 2008'den itibaren çoğu arabada.
- Ünite başına arıza belleği; arıza türü (kısa devre, açık devre, mantıksız ...), parça numarası, yazılım ve donanım sürümüyle birlikte.
- VAG, BMW ve Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault ve PSA için adres listeleri; diğer her şey için olağan adresler üzerinde tarama.
- F serisinden itibaren BMW ve Mini için 6F1 üzerinden BMW'nin kendi genişletilmiş adreslemesi, ayrıca tüm 128 teşhis adresi üzerinde tarama; arıza kodları ISTA'nın gösterdiği gibi BMW biçiminde (örn. 480A10).
- Ünite başına silme, uyarıyla birlikte.

![Tüm kontrol üniteleri](docs/screenshot-ecus.png)

### KKL kablosuyla VW ve Audi (KWP1281 ve KWP2000)
- Gaz kelebeği gövdesi adaptasyonu: 060 veya 098 ölçüm bloğundaki gerçek temel ayar.
- ABS, hava yastığı, gösterge paneli ve konfor dahil tüm kontrol ünitelerini tarama, yazdırılabilir raporla.
- Adaptasyon kanalları ve çıkış testleri, bilerek salt okunur.

### Üretici ve özel değerler
- Elektrikli araç aküleri: sağlık durumu, şarj, yayılımıyla hücre voltajları, sıcaklıklar; Hyundai Kona ve Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf ve Tesla Model 3 ile Y için (CAN veriyolunu dinleyerek); her değer belgelenmiş bir kaynakla.
- [OBDb](https://github.com/OBDb) projesinden yaklaşık 550 araç daha için üretici değerleri (akü, lastik basıncı, yağ, kilometre sayacı ve daha fazlası), algılanan araç için otomatik sunulur; her denklem OBDb'nin kendi çözücüsüyle denetlenir.
- Kendi değerlerinizi yaygın kullanılan Torque CSV biçiminde yükleyin; Torque denklemleri için eksiksiz bir değerlendirici var.

![EV aküsü](docs/screenshot-battery.png)

### Kaydet ve tekrar oynat
- Arabayla bir oturumu dosyaya kaydedin ve sonra arabasız oynatın; hata ararken işe yarar.

### Araba ya da adaptör olmadan
- Yerleşik bir demo araç: sürer, ısınır, birkaç arıza kayıtlıdır ve CAN üzerinden gerçek bir ELM327 gibi konuşur.
- Tüm kodların çevrimdışı sözlüğü, markaya göre filtrelenebilir.

### Görünüm ve his
- Odysync tarzında yeni bir arayüz: sakin, koyu, yumuşak geçişlerle.
- Tüm portlarda otomatik aramayla adım adım bağlanma; başarısız olursa bu bağlantı türü için kısa bir kontrol listesi (sürücü, eşleştirme, kontak, bit hızı, klon adaptörler) ve Caruspex'in zaten öğrendikleri.
- Başlangıç ve Profesyonel modu: yeni başlayanlar günlük sayfaları görür, servis araçları (KKL, adaptasyon, ham komutlar, adresler) Profesyonel modda belirir.
- 18 dil; ayarlar hemen uygulanır.
- Yeni sürüm çıktığında haber verir: başlangıçta GitHub'dan en yeni sürümü sorar ve başka hiçbir şey sormaz (sizinle ya da
  arabayla ilgili hiçbir veri gitmez); ayarlardan kapatılabilir.

## Desteklenen araçlar

OBD-II veya EOBD'li her araç: Avrupa'da 2001'den itibaren benzinli, 2004'ten itibaren dizel arabalar, ABD'de 1996'dan itibaren. Yaklaşık 1990'dan itibaren daha eski VW, Audi, Seat ve Skoda araçları KKL kablosuyla çalışır. Araçlar sayfası belirli bir araba için hangi yolun çalıştığını söyler.

| Dönem | Veriyolu | Örnekler |
|---|---|---|
| yaklaşık 1995'e kadar | üreticiye özgü | 2x2 fişleri üzerinden KKL'li VAG, diğerlerinde yalnızca atölye araçları |
| 1996'dan yaklaşık 2003'e | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| yaklaşık 2004'ten itibaren | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, modern Japon arabaları |
| ABD'de yaklaşık 2005'e kadar | SAE J1850 PWM ve VPW | Ford, Chevrolet |
| en yeni arabalar | UDS üzerinde OBD (SAE J1979-2) | ABD'de 2027 model yılından itibaren zorunlu, bazı yeni arabalarda şimdiden var; otomatik algılanır ve çevrilir |
| F serisinden itibaren BMW ve Mini | ENET (Ethernet üzerinden HSFZ) veya 6F1 adreslemeli D-CAN | her kontrol ünitesi |
| yaklaşık 2016'dan itibaren birçok araba | DoIP kablosuyla DoIP (ISO 13400) | Ethernet üzerinden OBD, otomatik bulunur |
| Tesla Model 3 ve Y | OBD yok: yayın CAN kareleri | dinleyerek akü, hücreler, sıcaklıklar, hız, kilometre sayacı (teşhis soketine adaptör kablosu) |

Saf elektrikli arabaların OBD-II emisyon verisi sunması gerekmez. Bazıları yine de kısmen yanıt verir; hiç yanıt vermeyen ama veriyolunda trafik olan arabalar için Caruspex dinlemek üzere bağlanır ve yerleşik paketlerin bildiği şeyleri okur (şimdilik Tesla Model 3 ve Y).

## Desteklenen adaptörler

- Herhangi bir **ELM327** uyumlu adaptör: USB, klasik Bluetooth (Windows ve Linux'ta eşleştirildikten sonra doğrudan RFCOMM üzerinden ya da COM portu aracılığıyla), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE ve olağan GATT hizmetlerine sahip diğer adaptörler; tarama ile
  bulunur, eşleştirme yok) veya TCP üzerinden Wi-Fi
- F serisinden itibaren BMW ve Mini için bir **ENET kablosu** (OBD'den RJ45'e): Caruspex arabayı ağda bulur
  (HSFZ, 6811 ve 6801 portları) ve araya adaptör yongası girmeden Ethernet üzerinden her kontrol ünitesini okur. Bilgisayarın
  otomatik bir adrese (169.254.x.x) ihtiyacı vardır; hiçbir DHCP yanıt vermediğinde Windows, macOS ve Linux bunu kendiliğinden atar.
  Burada da Caruspex yalnızca okur; kodlama ve yazılım yükleme E-Sys, ISTA veya ayar araçlarında kalır.
- Herhangi bir **KKL kablosu (VAG 409.1)**. FTDI veya CP2102 tabanlı kablolar K-Line üzerinde en ucuz
  klonlardan çok daha güvenilirdir; KWP1281 için orijinal bir FTDI FT232RL önerilir.

Delphi DS150E, Autocom veya VAS 5054A gibi özel arayüzler desteklenmez, ELM327 konuşmazlar.

| İşlev | ELM327 | KKL 409.1 |
|---|---|---|
| Motor arızalarını okuma ve silme | evet | evet |
| Canlı değerler, dondurulmuş kare, hazır olma | evet | evet |
| ABS, hava yastığı, gösterge paneli, konfor | **hayır** | evet |
| Üretici ölçüm blokları | **hayır** | evet |
| **Gaz kelebeği gövdesi adaptasyonu** | **hayır** | evet |

Bu bir donanım sınırıdır, yazılım sınırı değil: KWP1281, her baytın karşı taraftan
onaylandığı bir blok protokolüdür. ELM327 yazılımı yalnızca bütün istek/yanıt çiftlerini işler.
KKL kablosu K-Line'ı ham olarak geçirir, böylece Caruspex protokolü kendisi konuşabilir.

### İki VAG protokolü, otomatik algılanır

| Protokol | Tipik olduğu yer | Temel ayarlar |
|---|---|---|
| KWP1281 (blok protokolü) | 1.4 MPI motorlar | ayrı komut (blok 0x28) |
| KWP2000 (ISO 14230) | Bosch MED'li 1.6 FSI | ayrı teşhis oturumu, sonra blok okuma |

Adaptasyon kanalları ve çıkış testleri şu anda KWP1281 yolunda uygulanmıştır; KWP2000'de
uygulama sessizce başarısız olmak yerine bunu söyler.

Henüz uygulanmadı: VW TP 2.0 ile CAN üzerinden üretici teşhisi (yaklaşık 2004'ten itibaren araçlar).

## Teknoloji yığını

| Alan | Teknoloji |
|---|---|
| Dil | Rust (edition 2024) |
| Arayüz | React 19 ve TypeScript'li Tauri 2, Vite ile derlenir |
| Kitaplıklar | `serialport` (seri portlar), `btleplug` (Bluetooth LE), `libloading` (J2534 sürücüleri) |
| Yerelleştirme | Çekirdek ve arayüz için `translations/` içinde tek bir JSON dosya takımı, 18 dil |
| Derleme | Cargo çalışma alanı, arayüz için npm |
| Testler | Simüle edilmiş ünitelere, sahte ağ geçitlerine ve sahte bir J2534 sürücüsüne karşı `cargo test` |
| Araçlar | Python 3 (kod ve tablo üreticileri, çeviri denetimleri, ELM327 simülatörü) |
| Platformlar | Windows 10 ve 11, Linux, macOS |

## İndirme

Hazır paketler [sürümler sayfasında](https://github.com/SenseiIssei/Caruspex/releases):

| Sistem | Dosya | Notlar |
|---|---|---|
| Windows 10 ve 11, 64 bit | `Caruspex-<version>-windows-x64-setup.exe` | Yönetici hakları olmadan kurulur ve `caruspex_cli`'yi arama yoluna ekleyebilir. Kurulu bir 0.8'i yerinde günceller ve WebView2 çalışma zamanını yalnızca eksikse kurar |
| Windows 10 ve 11, 64 bit | `Caruspex-<version>-windows-x64.zip` | Taşınabilir, açın ve başlatın. Windows 10 ve 11'de zaten bulunan WebView2 çalışma zamanı gerekir |
| macOS 13 ve üzeri, Apple Silicon | `Caruspex-<version>-macos.dmg` | Uygulamayı Uygulamalar'a sürükleyin |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` yapıp başlatın |
| Debian ve Ubuntu, 64 bit | `Caruspex-<version>-linux-amd64.deb` | `sudo apt install ./Caruspex-<version>-linux-amd64.deb` ile kurun |
| Yalnızca komut satırı aracı | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Betikler ve sunucular için, arayüz yok |

Paketler henüz imzalı değil. Windows SmartScreen ilk başlatmada sorar ("Daha fazla bilgi", "Yine de çalıştır"); macOS'ta
uygulamayı ilk kez sağ tıklayıp "Aç" ile açın.

## Derleme

Gereksinimler: kararlı Rust araç zinciri (edition 2024, Rust 1.85 veya daha yeni), npm'li Node 24 ve
üreticiler ile çeviri denetimleri için Python 3. Linux'ta arayüzün ve adaptör bağlantılarının ihtiyaç duyduğu sistem paketleri
(Debian ve Ubuntu adlarıyla):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows için WebView2 çalışma zamanı gerekir (Windows 10 ve 11'in parçasıdır), macOS için ekstra bir şey gerekmez.

Arayüz bağımlılıklarını bir kez kurun, sonra komut satırı aracını ve uygulamayı derleyin:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` siz üzerinde çalışırken uygulamayı canlı yenilemeyle başlatır. Arayüzün tek başına,
Tauri penceresi olmadan, yerleşik demo araçla sıradan bir tarayıcıda çalışır:

```bash
npm --prefix app run dev
```

Demo araca hemen bağlanmak için `http://localhost:5287/?demo` adresini açın. `?lang=de` bir dil seçer,
`?theme=light` açık temayı, örneğin `?demo&lang=de&theme=light`. Tauri kabuğu olmadan
adaptör erişimi yoktur, bu yüzden tarayıcı sürümü yalnızca demo aracı bilir.

### Linux'ta seri port izinleri

Adaptöre `sudo` olmadan erişmek için:

```bash
sudo usermod -aG dialout $USER
```

Sonra oturumu kapatıp yeniden açın.

## Derleme çıktıları

| Çıktı | Açıklama |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | Tauri arayüzlü uygulama (`tauri build` yükleyicileri ve paketleri de `target/release/bundle/` içine yazar) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Aynı çekirdek üzerinde betiklenebilir komut satırı aracı, bkz. [Komut satırı aracı](#komut-satırı-aracı) |

### Paketler ve sürümler

`bash tools/package.sh` mevcut sistemin paketlerini `dist/` içine derler: Windows'ta yükleyici ve iki zip
(yükleyici [Inno Setup 6](https://jrsoftware.org/isinfo.php) gerektirir), macOS'ta dmg ve komut satırı arşivi,
Linux'ta AppImage, `.deb` ve komut satırı arşivi. `bash tools/smoke_test.sh` ardından bunları bir kullanıcı gibi kurar ya da
açar, iki programı da başlatır ve yeniden kaldırır. CI her çalışmada ikisini de yapar.

Bir sürüm bir etiketten gelir. `bash tools/release.sh <version>` onu hiçbir şey göndermeden kendi dalında hazırlar:
sürümü `Cargo.toml` ve `app/package.json` içinde ayarlar, `changelog/<version>.md` dosyasını oluşturur (Almanca ve İngilizce),
denetimleri, testleri, paketlemeyi ve duman testini çalıştırır ve README ekran görüntülerini yeniler (`tools/screenshots.sh de` ve
`tools/screenshots.sh en`). Pull request birleştirildikten sonra `v<version>` etiketini oluşturup gönderin. CI üç
sistemin hepsini derler ve sürümü paketler ve değişiklik listesiyle yayımlar.

CI (`.github/workflows/build.yml`) önce Linux'ta biçimlendirmeyi, üretilmiş tablo denetimini, çeviri denetimini, arayüz
derlemesini ve clippy'yi çalıştırır, ardından Windows, Linux ve macOS'u test eder, paketler ve duman testinden geçirir.
İkinci bir iş akışı (`.github/workflows/data-check.yml`) ayda bir kez yukarı akışta yeni VIN kodlarını, araçları ve DBC dosyalarını arar
ve yenilerle bir issue açar.

## Testler

```bash
cargo test --workspace
```

Yaklaşık 370 test birkaç saniyede çalışır, hepsi araba olmadan. ELM327 yanıt ayrıştırıcısını, arıza kodu çözmeyi,
PID dönüşümünü, araç profillerini ve VIN çözücüyü, demo araca karşı OBD-II ve UDS hizmetlerini,
simüle edilmiş kontrol ünitelerine karşı tam KWP1281 ve KWP2000 oturumlarını, sahte ağ geçitlerine karşı ENET (HSFZ) ve DoIP'i,
sahte bir sürücüye karşı SLCAN ve J2534 yolunu, kayıtları ve tekrar oynatmayı ve analizi (sağlık kontrolü, akü testi,
yolculuk bilgisayarı, performans zamanlayıcısı) kapsarlar. Komut satırı testleri derlenmiş `caruspex_cli`'yi çalıştırır ve çıktısını
`crates/caruspex-cli/tests/golden/` içindeki referans dosyalarla karşılaştırır; bunlar önceki C++ sürümünün aynı
komutlar için yazdırdığını içerir, böylece eski araca göre her fark ortaya çıkar.

CI'nin testlerin yanında çalıştırdığı denetimler:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

İlk ikisi biçimlendirme ve lint denetimidir. `tools/i18n_rust.py --strict`, çekirdekteki bir metin çeviri dosyalarında yoksa
ya da bir çeviri 18 dilden birinde eksikse başarısız olur; `I18N_STRICT=1` ayarlıyken arayüz derlemesi kendi
metinlerini aynı şekilde denetler. Üretilen tabloları yenilemek için `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` ve
`gen_pids.py` çalıştırın; depodakinden farklılarsa CI başarısız olur.

## Araba olmadan deneyin

En hızlı yol yerleşik demo araçtır: Bağlan sayfasında "Demo araç" seçin ya da şununla başlatın

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` uygulamanın tüm seçeneklerini listeler: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` ve `--size`.

Python simülatörü ya K-Line üzerinde Audi A2 motorunu ya da `--can` ile motoru ve şanzımanı olan
bir CAN arabasını oynatır (başlıklar, ISO-TP, kontrol ünitesi başına mod 06 ve 09):

```bash
python tools/mock_elm327.py --can
```

Sonra uygulamayı bağlayın (Windows'ta ikili dosya `target/release/caruspex.exe`'dir):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

Komut satırı aracı simülatöre karşı da çalışır:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Komut satırı aracı

`caruspex_cli <command>`; `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` veya `--replay FILE` ile:

| Komut | Ne yapar |
|---|---|
| `read` | Uyarı lambası, hazır olma ve arıza kodları; VIN ele veriyorsa marka için anlamıyla birlikte |
| `vin` | VIN, ne söylediği (üretici, ülke, model yılı, kontrol hanesi) ve kataloktan eşleşen araba |
| `freeze` | Her dondurulmuş kare, kayıtlı arıza başına bir tane |
| `health` | Sağlık kontrolü: arızalar, hazır olma, mod 06 ve canlı bir seri (`--seconds N`, varsayılan 15) |
| `probe` | Adaptör testi: ELM327'nin gerçekte ne olduğu, hangi komutları bildiği, gidiş dönüş süreleri |
| `uds` | CAN üzerinde UDS: her kontrol ünitesinin kimliği ve arıza belleği (kendi adresleriniz için `--ids A,B,...`) |
| `custom` | Yerleşik bir paketten (`--pack NAME`) veya bir Torque CSV'den (`--csv FILE`) üretici ve özel değerler, bir kez |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Canlı değerler, VIN ve ilk dondurulmuş kare, silme, hepsi kontrol ünitesi başına, seri portlar, KKL kablosu öz testi |
| `enet`, `interfaces` | Ağda BMW ve Mini arabalarını ve DoIP'li arabaları arar; bu bilgisayarın CAN arayüzlerini listeler (J2534 sürücüleri, SocketCAN) |
| `version`, `help` | Programın sürümü, komut ve seçeneklerin tam listesi |

`--json`, `read`, `vin`, `health`, `probe`, `freeze`, `uds` ve `custom` çıktısını betikler için JSON olarak yazdırır, `--record FILE`
adaptörle yapılan tüm oturumu `--replay` için kaydeder, `--fast-timing` adaptörü agresif uyarlamalı
zamanlamaya geçirir, `-v` her adaptör komutunu günlüğe yazar (`--json` ile birlikte stderr'e). `caruspex_cli help` her şeyi listeler.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Bu sayfadaki ekran görüntüleri demo araçtan gelir, uygulamanın kendisi aldı:
`--demo --page faults --shot shot.png --size 1440x920` bir sayfa açar, birkaç saniye sonra pencerenin görüntüsünü kaydeder
ve çıkar (`tools/screenshots.sh` tüm takımı alır).

## Proje yapısı

```
crates/caruspex-i18n/       Diller: kodda Almanca ve İngilizce, diğer 16'sı translations/ içinden
crates/caruspex-vehicles/   VIN çözücü ve araç kataloğu (markalar, modeller, nesiller, soketler, veriyolları)
crates/caruspex-data/       Arıza kodları, PID'ler, kontrol ünitesi adresleri, koddan bileşene eşleme
crates/caruspex-link/       Adaptöre giden yollar (seri, TCP, Bluetooth LE), ELM327 sürücüsü, adaptör testi
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 ve KWP1281 (K-Line, bayt bayt)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Kayıtlar ve tekrar oynatma, özel değerler (Torque CSV, PID paketleri), JSON yazıcı
crates/caruspex-analysis/   Sağlık kontrolü, sürüş döngüsü rehberi, yolculuk bilgisayarı, performans zamanlayıcısı, akü testi
crates/caruspex-demo/       Yerleşik demo araç: simüle edilmiş kontrol üniteli bir ELM327
crates/caruspex-cli/        Komut satırı aracı, referans dosya testleriyle birlikte
app/src/                    Arayüz: React 19 ve TypeScript (sayfalar, kabuk, durum, rapor, i18n)
app/src-tauri/              Tauri uygulaması: pencere, komutlar, güncelleme denetimi, komut satırı seçenekleri
translations/               Çekirdek ve arayüz için ortak çeviri dosyaları, 18 dil
data/                       Elle bakımı yapılan CSV olarak araç kataloğu, kontrol ünitesi ve PID kaynakları
tools/                      Üreticiler, çeviri denetimleri, paketleme, sürüm ve ekran görüntüsü betikleri, ELM327 simülatörü
docs/                       README ekran görüntüleri
packaging/                  Windows yükleyici betiği, Linux masaüstü dosyası, macOS Info.plist
changelog/                  Sürüm başına sürüm notları, Almanca ve İngilizce
```

Arıza kodu veritabanı üretilir: `tools/gen_dtc.py` düzenli EOBD kod ailelerini ve
`tools/dtc/` içindeki derlenmiş listeleri `crates/caruspex-data/src/generated/*.rs` dosyalarına açar. Veriyi değiştirdikten sonra yeniden çalıştırın:

```bash
python tools/gen_dtc.py
```

Araç kataloğu da aynı şekilde çalışır: `data/vehicles/` içindeki CSV dosyalarını düzenleyin, sonra şunu çalıştırın

```bash
python tools/gen_vehicles.py
```

Kontrol ünitesi adreslerinin ve PID paketlerinin `tools/gen_ecus.py` ve `tools/gen_pids.py` dosyaları vardır. Çeviriler
[translations/TRANSLATING.md](translations/TRANSLATING.md) dosyasında anlatılır; kodda bir metin ekledikten sonra
`python tools/i18n_rust.py` hâlâ neyin çevrilmesi gerektiğini listeler.

## Kod silme üzerine bir not

OBD mod 04 arıza belleğini, dondurulmuş kareyi ve hazır olma monitörlerini siler. Hemen ardından
araba, birkaç tam sürüş döngüsünü tamamlayana kadar emisyon testinden geçemez ve nedeni
hâlâ duran bir kod kısa bir sürüşten sonra geri gelir. Silmek onarım değildir.

## Sorumluluk reddi

Caruspex bir hobi projesidir ve garanti verilmeden sunulur. Kendi sorumluluğunuzda, yalnızca üzerinde çalışmanıza izin verilen
araçlarda ve asla sürüş sırasında kullanın. Tüm ürün ve marka adları ilgili
sahiplerinin ticari markalarıdır; Caruspex hiçbir araç üreticisiyle bağlantılı değildir ve onlarca onaylanmamıştır.

## Lisans

MIT, bkz. [LICENSE](LICENSE). Yüzlerce modelin üretici değerleri [OBDb](https://github.com/OBDb) projesinden dönüştürülmüştür ve OBDb gibi CC BY-SA 4.0 ile lisanslıdır: koddan ayrı olarak [data/obdb](data/obdb) içinde, lisans ve kaynak listesiyle ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)) birlikte durur.
Pakete dahil yazı tipleri SIL Open Font License 1.1 ile lisanslıdır:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) ve
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
