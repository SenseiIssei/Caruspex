# PEAK PCAN (Windows)

يمكن استخدام محوّلات CAN من PEAK-System (PCAN-USB، PCAN-USB FD، PCAN-USB Pro وغيرها) مباشرة على Windows، من دون تعريف J2534. على Linux هي واجهات [SocketCAN](socketcan.md).

## الاتصال

1. ثبّت تعريف الجهاز من PEAK. يأتي معه بالمكتبة `PCANBasic.dll` التي يحمّلها Caruspex.
2. في صفحة **اتصل** اختر **واجهة CAN**. كل قناة PCAN خلفها جهاز تظهر في القائمة من تلقاء نفسها؛ والقنوات التي يستخدمها برنامج آخر لا تظهر.
3. اختر معدل البت (500 kbit/s لـ OBD عبر CAN) واتصل.

## ما يعمل

OBD، وكل وحدات التحكم عبر UDS، والبحث المتوازي عن وحدات التحكم، وVW TP 2.0، والاستماع إلى القيم المذاعة. أما CAN FD عبر PCAN-Basic فلم يُدمج بعد.

## سطر الأوامر

```
caruspex_cli read --can pcan:USB1@500000
```
