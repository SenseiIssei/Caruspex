# एडैप्टर और इंटरफ़ेस

Caruspex ख़ुद पता लगाता है कि क्या लगा है और गाड़ी कौन सी भाषा बोलती है। गाड़ी तक पहुँचने का रास्ता चाहे जो हो, उस पर वही फ़ंक्शन काम करते हैं: OBD-II, UDS से हर कंट्रोल यूनिट, ब्रॉडकास्ट वैल्यू सुनना, रिकॉर्डिंग और रीप्ले।

| इंटरफ़ेस | उदाहरण | पेज |
|---|---|---|
| USB से ELM327 | सस्ती ELM327 केबल, Vgate, Veepeak | [USB से ELM327](elm327-usb.md) |
| ब्लूटूथ से ELM327 | क्लासिक ब्लूटूथ और Bluetooth LE प्लग, vLinker, Vgate iCar Pro | [ब्लूटूथ से ELM327](elm327-bluetooth.md) |
| वाई-फ़ाई से ELM327 | प्लग जो अपना ख़ुद का वाई-फ़ाई खोलते हैं | [वाई-फ़ाई से ELM327](elm327-wifi.md) |
| STN चिप | OBDLink SX, EX, MX+, CX | [OBDLink और STN](obdlink-stn.md) |
| KKL केबल (VAG 409.1) | FTDI या CP2102 केबल | [KKL केबल](kkl.md) |
| K+DCAN केबल | स्विच वाली BMW केबल | [K+DCAN केबल](kdcan.md) |
| ENET केबल | F सीरीज़ से BMW और Mini | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, candleLight वाला CANable, Raspberry Pi CAN हैट | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP केबल | लगभग 2016 से कई गाड़ियाँ | [DoIP](doip.md) |

## हर एक क्या कर सकता है?

साधारण ELM327 इंजन, गियरबॉक्स और, लगभग 2008 से ज़्यादातर गाड़ियों में, UDS से बाकी सभी कंट्रोल यूनिट के लिए काफ़ी है। यह Volkswagen ग्रुप का पुराना प्रोटोकॉल KWP1281 नहीं कर सकता, जो हर एक बाइट की पुष्टि करता है: लगभग 2004 से पहले की VW, Audi, Seat और Skoda के लिए आपको KKL केबल चाहिए।

रॉ CAN इंटरफ़ेस (J2534, SLCAN, SocketCAN, PCAN, Kvaser) Caruspex को सीधे CAN फ़्रेम देते हैं। तब ट्रांसपोर्ट प्रोटोकॉल वह ख़ुद संभालता है, जिससे कंट्रोल यूनिट की तेज़ पैरेलल खोज और लगभग 2004 से 2012 तक की Volkswagen ग्रुप की गाड़ियों के लिए VW TP 2.0 संभव होता है।

![एडैप्टर टेस्ट](screenshot-adapter.png)

## क्या मेरा एडैप्टर असली है?

**एडैप्टर टेस्ट** एडैप्टर से हर ELM327 वर्शन की कमांड पूछता है, जवाब का समय मापता है और ईमानदारी से बताता है कि इससे क्या नतीजा निकलता है: कई सस्ते एडैप्टर वर्शन 1.5 या 2.1 बताते हैं पर सिर्फ़ 1.0 की कमांड जानते हैं। Caruspex एडैप्टर में कभी कुछ नहीं लिखता।

> अब तक टेस्ट किया गया: STN2232 चिप वाले एक OBDLink ने एडैप्टर टेस्ट पास किया, बिना गाड़ी के। बाकी सभी एडैप्टर और इंटरफ़ेस उनके दस्तावेज़ों के हिसाब से बनाए गए हैं, पर अभी असली गाड़ी पर टेस्ट नहीं हुए हैं।
