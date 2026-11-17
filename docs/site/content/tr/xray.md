# Röntgen görünümü

**Röntgen görünümü**, bağlı aracı döndürebileceğiniz cam bir model olarak gösterir. İçeride motoru veya elektrik motorlarını, şanzımanı, depoyu veya yüksek voltaj aküsünü, egzozu, radyatörü, 12 V aküyü ve OBD soketini görürsünüz; hepsi tanınan aracın kasa tipine ve aktarma organlarına göre yerleştirilmiştir.

![Röntgen görünümü](screenshot-xray.png)

## Neler yanar

- **Arızalar** kendi bölgelerini renklendirir ve sorumlu kontrol ünitesi kırmızıya döner.
- **Canlı değerler** ait oldukları parçanın üzerinde etiket olarak durur: soğutma suyu radyatörde, devir motorda, voltaj aküde.
- Arama ile bulunan **kontrol üniteleri** genellikle bulundukları yerde durur ve veri hatlarıyla (aktarma organları CAN'i, konfor CAN'i, teşhis CAN'i, K-Line) birbirine bağlanır. Caruspex okurken küçük paketler sorgulanan üniteye doğru gider.
- **Aracı kontrol et**, kamerayı her bulguya sırayla uçurur.

Fare veya parmakla döndürün, tekerlek veya iki parmakla yakınlaştırın; klavye de çalışır. Katmanlar (kasa, aktarma organları, soğutma, yakıt veya yüksek voltaj, elektrik, ağ) tek tek kapatılabilir.

## Model ne kadar doğru?

Şekil, biliniyorsa kuşağın gerçek ölçülerinden (Wikidata'dan veya üreticinin teknik verilerinden), bilinmiyorsa kasa tipi ve dönem için tipik değerlerden gelir. Tam modelin bir kopyası değildir: üreticilerin tasarım verileri ve kablo şemaları serbestçe erişilebilir değildir. Sayfa şeklin nereden geldiğini söyler.

Aracınızın glTF (`.glb` veya `.gltf`) biçiminde bir 3D modeli varsa onu **Kendi 3D modelini yükle** ile yükleyebilirsiniz; iç kısım yerleşimden gelmeye devam eder.

## 3D olmadan

WebGL olmayan bilgisayarlarda sayfa, aynı bilgileri içeren düz bir yan görünüme geçer.
