# Kvaser (Windows)

يمكن استخدام محوّلات CAN من Kvaser (Leaf، U100، Memorator وغيرها) مباشرة على Windows من خلال مكتبة Kvaser المسماة CANlib. على Linux هي واجهات [SocketCAN](socketcan.md).

## الاتصال

1. ثبّت تعريفات Kvaser. تأتي معها `canlib32.dll` التي يحمّلها Caruspex.
2. في صفحة **اتصل** اختر **واجهة CAN**. كل قناة Kvaser خلفها جهاز تظهر في القائمة؛ والقنوات الافتراضية للتعريف لا تظهر.
3. اختر معدل البت (500 kbit/s لـ OBD عبر CAN) واتصل.

## ما يعمل

OBD، وكل وحدات التحكم عبر UDS، والبحث المتوازي عن وحدات التحكم، وVW TP 2.0، والاستماع إلى القيم المذاعة. أما CAN FD عبر CANlib فلم يُدمج بعد.

## سطر الأوامر

```
caruspex_cli read --can kvaser:0@500000
```
