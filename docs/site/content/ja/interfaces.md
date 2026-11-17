# アダプターとインターフェース

Caruspex は、何がつながっているか、車が何を話すかを自分で見分けます。どんな方法で車につないでも、同じ機能が使えます：OBD-II、UDS によるすべてのコントロールユニット、ブロードキャスト値の受信、記録と再生。

| インターフェース | 例 | ページ |
|---|---|---|
| USB 接続の ELM327 | 安い ELM327 ケーブル、Vgate、Veepeak | [USB 接続の ELM327](elm327-usb.md) |
| Bluetooth 接続の ELM327 | クラシック Bluetooth と Bluetooth LE のプラグ、vLinker、Vgate iCar Pro | [Bluetooth 接続の ELM327](elm327-bluetooth.md) |
| Wi-Fi 接続の ELM327 | 自分で Wi-Fi を作るプラグ | [Wi-Fi 接続の ELM327](elm327-wifi.md) |
| STN チップ | OBDLink SX、EX、MX+、CX | [OBDLink と STN](obdlink-stn.md) |
| KKL ケーブル（VAG 409.1） | FTDI または CP2102 のケーブル | [KKL ケーブル](kkl.md) |
| K+DCAN ケーブル | スイッチ付きの BMW 用ケーブル | [K+DCAN ケーブル](kdcan.md) |
| ENET ケーブル | F シリーズ以降の BMW と Mini | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0、Mongoose、VCX Nano、Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable、USBtin、CANUSB | [SLCAN](slcan.md) |
| SocketCAN（Linux） | PEAK、Kvaser、candleLight の CANable、Raspberry Pi の CAN HAT | [SocketCAN](socketcan.md) |
| PEAK PCAN（Windows） | PCAN-USB、PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser（Windows） | Kvaser Leaf、U100 | [Kvaser](kvaser.md) |
| DoIP ケーブル | 2016 年ごろ以降の多くの車 | [DoIP](doip.md) |

## それぞれ何ができる？

普通の ELM327 で、エンジン、トランスミッション、そして 2008 年ごろ以降のほとんどの車では UDS によるほかのすべてのコントロールユニットまで読めます。ただし、1 バイトごとに確認応答を返す古い Volkswagen グループのプロトコル KWP1281 には対応できません：2004 年ごろより前の VW、Audi、Seat、Skoda には KKL ケーブルが必要です。

生の CAN インターフェース（J2534、SLCAN、SocketCAN、PCAN、Kvaser）は、Caruspex にむき出しの CAN フレームを渡します。転送プロトコルは Caruspex 自身が処理するので、コントロールユニットの高速な並列検索や、2004 年から 2012 年ごろの Volkswagen グループ車向けの VW TP 2.0 が使えます。

![アダプターのテスト](screenshot-adapter.png)

## 自分のアダプターは本物？

**アダプターのテスト**は、ELM327 の各バージョンのコマンドをアダプターに問い合わせ、応答時間を測り、そこから分かることを正直に伝えます：安いアダプターの多くはバージョン 1.5 や 2.1 を名乗りますが、1.0 のコマンドしか分かりません。Caruspex がアダプターに何かを書き込むことはありません。

> これまでにテストしたもの：STN2232 チップの OBDLink がアダプターのテストに合格しました。ただし車にはつないでいません。ほかのアダプターとインターフェースはすべて資料に従って作られていますが、まだ実車ではテストしていません。
