# المحوّلات والواجهات

يكتشف Caruspex بنفسه ما هو موصول وما اللغة التي تتحدثها السيارة. أيًا كان طريقك إلى السيارة، تعمل عليه الوظائف نفسها: OBD-II، وكل وحدات التحكم عبر UDS، والاستماع إلى القيم المذاعة، والتسجيل والإعادة.

| الواجهة | أمثلة | الصفحة |
|---|---|---|
| ELM327 عبر USB | كابلات ELM327 الرخيصة، Vgate، Veepeak | [ELM327 عبر USB](elm327-usb.md) |
| ELM327 عبر البلوتوث | قوابس بلوتوث الكلاسيكي وBluetooth LE، vLinker، Vgate iCar Pro | [ELM327 عبر البلوتوث](elm327-bluetooth.md) |
| ELM327 عبر واي فاي | قوابس تفتح شبكة واي فاي خاصة بها | [ELM327 عبر واي فاي](elm327-wifi.md) |
| شرائح STN | OBDLink SX، EX، MX+، CX | [OBDLink وSTN](obdlink-stn.md) |
| كابل KKL (VAG 409.1) | كابلات FTDI أو CP2102 | [كابل KKL](kkl.md) |
| كابل K+DCAN | كابلات BMW بمفتاح | [كابل K+DCAN](kdcan.md) |
| كابل ENET | BMW وMini بدءًا من الفئة F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0، Mongoose، VCX Nano، Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable، USBtin، CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK، Kvaser، CANable مع candleLight، لوحات CAN لـ Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB، PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf، U100 | [Kvaser](kvaser.md) |
| كابل DoIP | سيارات كثيرة من نحو 2016 | [DoIP](doip.md) |

## ماذا يستطيع كل منها؟

يكفي ELM327 العادي للمحرك وناقل الحركة، وفي معظم السيارات من نحو 2008 لكل وحدات التحكم الأخرى عبر UDS. لكنه لا يستطيع بروتوكول مجموعة فولكس فاجن القديم KWP1281، الذي يؤكد استلام كل بايت على حدة: لسيارات VW وAudi وSeat وSkoda قبل نحو 2004 تحتاج إلى كابل KKL.

واجهات CAN الخام (J2534، SLCAN، SocketCAN، PCAN، Kvaser) تعطي Caruspex إطارات CAN المجردة. عندها يتولى بنفسه بروتوكول النقل، وهذا يتيح بحثًا متوازيًا سريعًا عن وحدات التحكم وVW TP 2.0 لسيارات مجموعة فولكس فاجن من نحو 2004 إلى 2012.

![اختبار المحوّل](screenshot-adapter.png)

## هل محوّلي أصلي؟

**اختبار المحوّل** يسأل المحوّل عن أوامر كل إصدار من ELM327، ويقيس أزمنة الاستجابة، ويقول بصراحة ما يُستنتج من ذلك: كثير من المحوّلات الرخيصة تعلن الإصدار 1.5 أو 2.1 لكنها لا تعرف إلا أوامر الإصدار 1.0. لا يكتب Caruspex أي شيء في المحوّل أبدًا.

> ما اختُبر حتى الآن: OBDLink بشريحة STN2232 اجتاز اختبار المحوّل، من دون سيارة. كل المحوّلات والواجهات الأخرى مبنية وفق توثيقها لكنها لم تُختبر بعد على سيارة حقيقية.
