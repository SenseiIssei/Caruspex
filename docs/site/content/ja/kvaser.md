# Kvaser（Windows）

Kvaser の CAN アダプター（Leaf、U100、Memorator など）は、Windows では Kvaser のライブラリ CANlib を通じて直接使えます。Linux では [SocketCAN](socketcan.md) のインターフェースになります。

## 接続

1. Kvaser のドライバーをインストールします。`canlib32.dll` が入り、Caruspex がそれを読み込みます。
2. **接続** ページで **CAN インターフェース**を選びます。デバイスがつながっている Kvaser のチャンネルがすべて一覧に表示されます。ドライバーの仮想チャンネルは除外されます。
3. ビットレート（CAN 上の OBD なら 500 kbit/s）を選んで、接続します。

## できること

OBD、UDS によるすべてのコントロールユニット、コントロールユニットの並列検索、VW TP 2.0、ブロードキャスト値の受信。CANlib 経由の CAN FD はまだ組み込まれていません。

## コマンドライン

```
caruspex_cli read --can kvaser:0@500000
```
