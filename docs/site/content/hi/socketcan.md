# SocketCAN (Linux)

SocketCAN, Linux कर्नेल की CAN लेयर है। कर्नेल ड्राइवर वाला हर CAN एडैप्टर इसके ज़रिए इस्तेमाल किया जा सकता है: PEAK PCAN-USB, Kvaser, candleLight फ़र्मवेयर वाला CANable, Raspberry Pi CAN हैट और बहुत सारे।

## सेटअप

इंटरफ़ेस को गाड़ी के बिट रेट के साथ चालू करें (CAN पर OBD के लिए 500 kbit/s):

```
sudo ip link set can0 up type can bitrate 500000
```

सबसे नई गाड़ियों पर CAN FD के लिए, डेटा बिट रेट के साथ भी:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## कनेक्ट करना

**जुड़ें** पेज पर **CAN इंटरफ़ेस** चुनें। कंप्यूटर का हर CAN इंटरफ़ेस सूची में है (`can0`, `can1` वगैरह)। उसे चुनें, बिट रेट चुनें और, CAN FD के लिए, CAN FD बॉक्स चालू करें।

## क्या काम करता है

OBD, UDS से हर कंट्रोल यूनिट, कंट्रोल यूनिट की पैरेलल खोज, VW TP 2.0, ब्रॉडकास्ट वैल्यू सुनना और CAN FD।

## कमांड लाइन

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
