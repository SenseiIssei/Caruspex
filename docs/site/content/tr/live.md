# Canlı değerler

**Canlı değerler** sayfası (profesyonel mod) sensörlerin şu anda ölçtüklerini gösterir. Başlangıç modunda **Sürüş modu** en önemli değerleri büyük yazıyla gösterir.

![Canlı değerler](screenshot-live.png)

## Neler var

- SAE J1979'un PID C4'e kadar her mod 01 değeri: 300'den fazla okumaya sahip 145 PID, gruplanmış ve aranabilir. Aracınızın hangilerini sunduğunu araç kendisi söyler.
- Kilometre sayacı, vites, yağ sıcaklığı, yakıt tüketimi, tork, turbo basıncı, egzoz gazı sıcaklıkları, partikül filtresi, AdBlue, NOx ve hibrit akü, araç bunları bildirdiği ölçüde.
- Aynı anda birkaç değer için kayan bir grafik. Uzun zaman aralıklarında (bir saate kadar) grafik eski noktaları seyreltir ama her tepeyi ve her çukuru korur.
- CSV'ye kayıt.

## Üretici değerleri ve kendi değerleriniz

- **Elektrikli araç aküleri**: Hyundai Kona ve Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf ve Tesla Model 3 ve Y için (CAN hattını dinleyerek) sağlık durumu, şarj, sapmasıyla birlikte hücre voltajları ve sıcaklıklar, **Elektrikli araç aküsü** sayfasında.
- Yaklaşık 550 araç daha için **OBDb değerleri** (lastik basıncı, yağ, kilometre ve daha fazlası), tanınan araç için kendiliğinden sunulur.
- **Genişletilmiş teşhis oturumu**: Volkswagen grubu dizellerinin bazı değerlerini, örneğin ölçülen is kütlesini ve son rejenerasyondan bu yana gidilen mesafeyi, motor kontrol ünitesi yalnızca genişletilmiş teşhis oturumunda verir. Profesyonel modda Caruspex bu tür değerler sorgulandığı sürece motoru bu oturuma alır, yalnızca okumak için, onları **Oturum 03** ile işaretler ve sonra geri alır. Bu, ayarlarda **Üretici değerleri** altında kapatılabilir. Kontrol ünitesi reddederse, örneğin sürüş sırasında, nedeni değerin yanında görünür.
- Yaygın Torque CSV biçiminde **kendi değerleriniz**, formüller dahil.

![Elektrikli araç aküsü](screenshot-battery.png)

## Ne kadar hızlı?

Saniyede kaç değer geldiği araca ve adaptöre bağlıdır. CAN'li araçlar hızlı yanıt verir; K-Line'daki araçlar doğası gereği yavaştır. Bir OBDLink, Caruspex'in tek seferde 24'e kadar değer sormasını sağlar (bkz. [OBDLink ve STN](obdlink-stn.md)); ham bir CAN arayüzü en hızlı yoldur.

## Kayıt ve yeniden oynatma

Araçla yapılan bütün bir oturum bir dosyaya kaydedilebilir ve daha sonra araç olmadan yeniden oynatılabilir; arıza ararken kullanışlıdır. **Kaydı paylaş** ile bir kayıt, kişisel veriler olmadan proje için hazırlanabilir, bkz. [gizlilik](privacy.md).
