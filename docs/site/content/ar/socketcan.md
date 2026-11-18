# SocketCAN (Linux)

SocketCAN هي طبقة CAN في نواة Linux. يمكن استخدام أي محوّل CAN له تعريف في النواة من خلالها: PEAK PCAN-USB، وKvaser، وCANable ببرنامج candleLight الثابت، ولوحات CAN لـ Raspberry Pi وغيرها كثير.

## الإعداد

شغّل الواجهة بمعدل البت الخاص بالسيارة (500 kbit/s لـ OBD عبر CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

لـ CAN FD في أحدث السيارات، مع معدل بت للبيانات أيضًا:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## الاتصال

في صفحة **اتصل** اختر **واجهة CAN**. كل واجهات CAN في الحاسوب موجودة في القائمة (`can0`، `can1` وهكذا). اختر الواجهة، وحدد معدل البت، ولـ CAN FD فعّل خانة CAN FD.

## ما يعمل

OBD، وكل وحدات التحكم عبر UDS، والبحث المتوازي عن وحدات التحكم، وVW TP 2.0، والاستماع إلى القيم المذاعة، وCAN FD.

## سطر الأوامر

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
