# SLCAN（Lawicel）

SLCAN は、CAN スティック向けの、シリアルポート上のシンプルなテキストプロトコルです：slcan ファームウェアの CANable、USBtin、Lawicel CANUSB、そして多くの自作デバイス。Windows、Linux、macOS で使えます。

## 接続

1. スティックをパソコンにつなぎ、ケーブルで OBD コネクターにつなぎます：CAN high は 6 番ピン、CAN low は 14 番ピン、グラウンドは 4 番と 5 番ピンです。
2. **接続** ページで **CAN インターフェース**を選びます。SLCAN スティックはポートの自動検索で見つかり、一覧に表示されます。
3. ビットレート（CAN 上の OBD なら 500 kbit/s）を選んで、接続します。

## できること

ほかの生の CAN インターフェースと同じです：OBD、UDS によるすべてのコントロールユニット、コントロールユニットの並列検索、VW TP 2.0、ブロードキャスト値の受信。SLCAN では CAN FD は使えません。

## ヒント

- candleLight ファームウェアの CANable は SLCAN デバイスではありません：Linux では [SocketCAN](socketcan.md) のインターフェースとして表示されます。
- OBD コネクター側にスティックの終端抵抗は要りません。車が自分で持っています。

## コマンドライン

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
