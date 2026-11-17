# OBDLink と STN チップ

OBDLink のアダプター（SX、EX、MX+、CX、LX）は、OBD Solutions 製の STN チップを使っています。ELM327 にできることはすべてでき、さらにそれ以上もできます。Caruspex はこれを自分で見分けて、追加機能を使います。

## Caruspex が使う機能

- **まとめて要求**：Caruspex は一度に 6 個ではなく最大 24 個のライブ値を要求します。Bluetooth では、これで 1 巡の値の取得がほぼ 2 倍速くなります。
- **受信**：ブロードキャスト値（例えば Tesla のもの）は STN 独自のコマンドで記録します。このコマンドは、決まった数のメッセージを受け取ると自分で止まります。
- **Ford MS-CAN と GM Single-Wire-CAN**：Ford と GM のボディ系コントロールユニットは、普通の診断用 CAN にはつながっていません。追加のトランシーバーを持つアダプターなら、**すべてのコントロールユニット** ページに「Ford：MS-CAN 上のボディユニット」または「GM：Single-Wire-CAN 上のボディユニット」が表示されます。

| チップ | アダプター | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 から STN2232 | OBDLink EX | 対応 | 非対応 |
| STN1150 から 1152、1170、2120、2255、2256 | OBDLink MX+ など | 対応 | 対応 |
| その他 | OBDLink SX、LX、CX | 非対応 | 非対応 |

## 接続

OBDLink のアダプターは、ほかの ELM327 と同じようにつなぎます：USB（EX と SX）、Bluetooth（MX+、LX、CX）、または [USB 接続の ELM327](elm327-usb.md) と [Bluetooth 接続の ELM327](elm327-bluetooth.md) の説明どおりに。

## テスト済み

STN2232 チップの OBDLink がアダプターのテストに合格しました：チップ、ファームウェア、コマンドが正しく認識されました。まだ車では使っていないので、まとめて要求、MS-CAN、SW-CAN は実際のハードウェアではまだ確認できていません。
