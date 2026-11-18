# PEAK PCAN（Windows）

PEAK-System の CAN アダプター（PCAN-USB、PCAN-USB FD、PCAN-USB Pro など）は、J2534 ドライバーなしで Windows で直接使えます。Linux では [SocketCAN](socketcan.md) のインターフェースになります。

## 接続

1. PEAK のデバイスドライバーをインストールします。ライブラリ `PCANBasic.dll` が入り、Caruspex がそれを読み込みます。
2. **接続** ページで **CAN インターフェース**を選びます。デバイスがつながっている PCAN のチャンネルがすべて自動で一覧に表示されます。ほかのプログラムが使用中のチャンネルは除外されます。
3. ビットレート（CAN 上の OBD なら 500 kbit/s）を選んで、接続します。

## できること

OBD、UDS によるすべてのコントロールユニット、コントロールユニットの並列検索、VW TP 2.0、ブロードキャスト値の受信。PCAN-Basic 経由の CAN FD はまだ組み込まれていません。

## コマンドライン

```
caruspex_cli read --can pcan:USB1@500000
```
