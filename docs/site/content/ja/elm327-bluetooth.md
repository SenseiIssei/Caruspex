# Bluetooth 接続の ELM327

ケーブルのない小さなプラグです。Bluetooth には 2 種類あり、Caruspex はどちらにも対応しています。

## Bluetooth LE

Vgate iCar Pro BLE、vLinker、OBDLink CX、Veepeak BLE のような新しいアダプターは Bluetooth Low Energy を使います。**ペアリングは不要**です：**接続** ページで **Bluetooth**を選び、**Bluetooth デバイスを検索**をクリックして、一覧からアダプターを選びます。Windows、Linux、macOS、Android で使えます。

## クラシック Bluetooth

古いアダプターや安いアダプターの多くは、シリアルプロファイルのクラシック Bluetooth を使います。

1. システムの Bluetooth 設定で、アダプターを一度ペアリングします。PIN はたいてい `1234` か `0000` です。
2. Caruspex で **Bluetooth**を選び、ペアリング済みのアダプターを選びます。一覧には、そのデバイスが LE か、クラシックか、両方かが表示されます。

Windows と Linux では、Caruspex はペアリング済みのクラシックアダプターに直接つなぎます。また、Windows はアダプター用の COM ポートも作るので、[USB 接続の ELM327](elm327-usb.md) と同じように使えます。macOS では、クラシック Bluetooth アダプターはシステムがシリアルポートを用意した場合にしか使えません。macOS では Bluetooth LE のアダプターのほうがおすすめです。

## Android の場合

- Bluetooth LE アダプターはここでもペアリング不要です：**Bluetooth** を選んで検索すると、一覧に各デバイスの電波強度が表示されます。
- クラシックアダプターは Android の設定で一度ペアリングすると、一覧に表示されます。
- Android 12 以降では、付近のデバイスへのアクセスを一度だけ求められます。Android 11 までは、Caruspex が位置情報を使えて、位置情報がオンのときだけ Bluetooth LE アダプターが見つかります。Caruspex は位置情報を読み取りません。ペアリング済みのクラシックアダプターは位置情報なしでも使えます。

## ヒント

- 使わないときはアダプターを抜いておきましょう：安いアダプターの多くは、車のバッテリーを消耗させ続けます。
- アダプターが表示されないときは、パソコンの Bluetooth をいったんオフにしてからオンに戻し、スマホのアプリが同時につながっていないか確認してください。
- Bluetooth はケーブルより遅いです。OBDLink なら Caruspex が複数の値を一度に要求するので、その差はほとんど埋まります（[OBDLink と STN](obdlink-stn.md) を参照）。
