# 安全、商標、ライセンス

## 安全と責任

Caruspex は自由な趣味のプロジェクトであり、[MIT ライセンス](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) にあるとおり無保証で提供されます。ご利用は自己責任でお願いします。

- 運転中は絶対に操作しないでください。設定は停車中に行い、ライブ値は同乗者に見てもらってください。
- 故障の消去、アクチュエータテスト、基本設定は車に何かをさせます。Caruspex はそれぞれの前に確認します。停車して車を固定した状態で、作業を許された車両でのみ行ってください。
- Caruspex は整備工場の代わりにはなりません。故障コードは症状を示すもので、原因を示すとは限りません。ブレーキ、エアバッグ、ステアリングなど安全に関わるものは専門家に任せてください。
- 消去は修理ではありません。消去した故障やリセットされたレディネスモニターにより、排ガス検査や車検に通らないことがあります。
- Caruspex は走行距離、イモビライザー、キー、排ガス浄化装置、タコグラフを変更しません。また、そうした変更を行う貢献は受け付けません。走行距離計、排ガス浄化装置、タコグラフの改ざんは多くの国で違法です。

## 商標

自動車メーカー、車種、アダプター、プログラム、規格の名称（例：Volkswagen、Audi、BMW、Mercedes-Benz、Tesla、OBDLink、ELM327、VCDS、ISTA）は、それぞれの所有者の商標です。Caruspex がどの車、アダプター、インターフェースで動作するかを示すためだけに記載しています。Caruspex は独立したプロジェクトであり、自動車メーカー、アダプターメーカー、ソフトウェアベンダーのいずれとも提携しておらず、承認や後援も受けていません。アプリとこのリポジトリはこれらの企業のロゴを使用していません。

## ライセンスと出典

コードは MIT ライセンスです。[LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) を参照してください。プログラムに含まれるライブラリとそのライセンスは [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md) にあり、アプリでは設定画面に表示されます。プログラムに組み込まれたデータ：

- 約 550 車種のメーカー値。[OBDb](https://github.com/OBDb) から変換したもので、OBDb と同じく CC BY-SA 4.0 です。コードとは別に [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) に置かれ、ライセンスと出典一覧（[ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)）が付いています。
- CAN 信号の定義は [opendbc](https://github.com/commaai/opendbc)（comma.ai）と [model3dbc](https://github.com/joshwardell/model3dbc) から、MIT ライセンス（[data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc)、[data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)）。
- Hyundai Ioniq と Kona、Kia e-Niro、Nissan Leaf、Renault Zoe のバッテリー値は OBDb（CC BY-SA 4.0、このパックも同じ）、[Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3)、[evDash](https://github.com/nickn17/evDash)（どちらも MIT）に基づきます：[data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md))。
- エンジンコードは Wikipedia の記事から、CC BY-SA 4.0（[data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines)、各行に記事と版を記載）。
- VIN のメーカーコードは [NHTSA vPIC](https://vpic.nhtsa.dot.gov)（パブリックドメイン）と [Wikidata](https://www.wikidata.org)（CC0）で確認済み。[Wikibooks の WMI 一覧](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) にしかないコードは CC BY-SA 4.0 で、[data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi) に分けて置いています。
- 車両の寸法は [Wikidata](https://www.wikidata.org) から、CC0。
- SIL Open Font License 1.1 のフォント：JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt))。
