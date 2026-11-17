# SocketCAN（Linux）

SocketCAN は Linux カーネルの CAN レイヤーです。カーネルドライバーのある CAN アダプターなら、どれでもこれを通して使えます：PEAK PCAN-USB、Kvaser、candleLight ファームウェアの CANable、Raspberry Pi の CAN HAT など多数。

## セットアップ

車のビットレート（CAN 上の OBD なら 500 kbit/s）でインターフェースを立ち上げます：

```
sudo ip link set can0 up type can bitrate 500000
```

最新の車の CAN FD では、データビットレートも指定します：

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## 接続

**接続** ページで **CAN インターフェース**を選びます。パソコンのすべての CAN インターフェースが一覧にあります（`can0`、`can1` など）。それを選び、ビットレートを選び、CAN FD の場合は CAN FD のチェックをオンにします。

## できること

OBD、UDS によるすべてのコントロールユニット、コントロールユニットの並列検索、VW TP 2.0、ブロードキャスト値の受信、CAN FD。

## コマンドライン

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
