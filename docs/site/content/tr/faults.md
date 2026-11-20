# Arızalar

**Arıza belleği** sayfası kontrol ünitelerinin kaydettiklerini okur ve sade bir dille açıklar.

![Arıza belleği](screenshot-faults.png)

## Ne görürsünüz

- **Kayıtlı, beklemede olan ve kalıcı** kodlar. Birden fazla kontrol ünitesi varsa her kod, onu kimin bildirdiğini gösterir (motor, şanzıman).
- Yaklaşık 3.400 standart kod (P, C, B, U) ve VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai ve Kia, Subaru ve diğerlerinin üretici kodları. Bir P1 kodu, tanınan markanın kastettiği şekilde okunur.
- Her kod için bir **trafik ışığı** ve herkesin ilk sorduğu soru: yola devam edebilir miyim?
- Olası nedenler, kendiniz kontrol edebileceğiniz şeyler ve kabaca bir maliyet sınıfı.
- Parçanın nerede olduğu: aracın şeffaf görünümü ya da [röntgen görünümü](xray.md).

## Tüm kontrol üniteleri

Profesyonel modda **Tüm kontrol üniteleri** sayfası yalnızca motoru değil, her kontrol ünitesini sorgular: ABS, hava yastığı, gösterge paneli, klima ve daha fazlası. Her arıza, arıza türü (kısa devre, açık devre, mantıksız sinyal), parça numarası ve yazılım sürümüyle birlikte gelir. Eski VW grubu araçlar bir [KKL kablosu](kkl.md) üzerinden okunur.

## Şimdi ne yapmalıyım?

**Ne yapmalı?** sayfası Caruspex'in bulduğu her şeyi üç adıma ayırır: hemen durun, yakında servise, bekleyebilir. Her maddede nedenini anlatan tek bir sade cümle, kabaca bir maliyet aralığı ve serviste gösterilecek bir not vardır. OBD, motor arıza lambasının yanıp söndüğünü veya kırmızı bir uyarı lambasının yandığını göremez, bu yüzden iki kutu bunu size sorar; teklemeyle birlikte yanıp sönen bir lamba, maddeyi "hemen durun" adımına taşır.

![Şimdi ne yapmalıyım](screenshot-todo.png)

## Kodları silme

Silme, her kontrol ünitesi için ayrı ayrı ve açık bir uyarıyla yapılabilir; Caruspex hemen ardından belleği yeniden okur.

> Silmek onarım değildir. Hazır olma monitörlerini de sıfırlar: araç birkaç tam sürüş döngüsünü tamamlayana kadar emisyon testinden geçemez ve nedeni hâlâ duran bir kod kısa bir sürüşten sonra geri gelir.

## Dışa aktarma

Liste CSV olarak, kontrol raporu ise PDF olarak kaydedilebilir.
