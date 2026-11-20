# 安全、商标与许可证

## 安全与责任

Caruspex 是一个自由的业余项目，正如 [MIT 许可证](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) 所述，不提供任何担保。使用风险由你自行承担。

- 切勿在驾驶时操作。请在停车时完成设置；实时数据可以由乘客查看。
- 清除故障、执行元件测试和基本设定会让车辆执行某些动作。Caruspex 在每一项之前都会询问；只能在车辆停稳并固定后进行，并且只能用于你有权操作的车辆。
- Caruspex 不能代替修理厂。故障码指出的是症状，不一定是原因。制动、安全气囊、转向以及一切与安全相关的部分都应交给专业人员。
- 清除不等于修理。清除的故障和被重置的就绪监测可能导致排放检测或车辆年检不通过。
- Caruspex 不会修改里程、防盗系统、钥匙、排放控制或行驶记录仪，本项目也不接受这类修改。篡改里程表、排放控制或行驶记录仪在许多国家是违法的。

## 商标

汽车制造商、车型、适配器、程序和标准的名称（例如 Volkswagen、Audi、BMW、Mercedes-Benz、Tesla、OBDLink、ELM327、VCDS 或 ISTA）是其各自所有者的商标。提到它们只是为了说明 Caruspex 能配合哪些车辆、适配器和接口使用。Caruspex 是一个独立项目，与任何汽车制造商、适配器制造商或软件供应商均无关联，也未获得其认可或赞助。应用和本仓库不使用这些公司的标志。

## 许可证与来源

代码采用 MIT 许可证，见 [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE)。程序中包含的库及其许可证列在 [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md) 中；应用在设置中显示它们。内置于程序中的数据：

- 约 550 款车辆的制造商数据，转换自 [OBDb](https://github.com/OBDb)，与 OBDb 一样采用 CC BY-SA 4.0：与代码分开存放在 [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb)，附有许可证和来源列表（[ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)）。
- CAN 信号定义来自 [opendbc](https://github.com/commaai/opendbc)（comma.ai）和 [model3dbc](https://github.com/joshwardell/model3dbc)，MIT 许可证（[data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc)、[data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)）。
- 发动机代码来自维基百科条目，CC BY-SA 4.0（[data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines)，每行都注明条目和版本）。
- 部分 VIN 制造商代码依据 [维基教科书上的 WMI 列表](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29)，CC BY-SA 4.0。
- 车辆尺寸来自 [维基数据](https://www.wikidata.org)，CC0。
- 采用 SIL Open Font License 1.1 的字体：JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt))。
