# 常见问题

## Caruspex 要花钱吗？

不要。Caruspex 是基于 MIT 许可证的自由软件，没有广告，不用注册账号，也没有付费附加功能。

## 我该买哪个适配器？

对大多数人来说，带 FTDI 或 CH340 芯片的 USB ELM327，或者一个 OBDLink（蓝牙或 USB），是最好的起点。大约 2004 年以前的 VW、Audi、Seat 和 Skoda，再加一根 [KKL 线](kkl.md)；F 系列起的 BMW 和 Mini，用一根 [ENET 线](enet.md)。[接口一览](interfaces.md)对它们做了比较。

## 我的车能用吗？

凡是带 OBD-II 或 EOBD 的车，发动机部分都能用：欧洲 2001 年起的汽油车和 2004 年起的柴油车，美国 1996 年起的车。大约 1990 年起的 VW 集团老车可以通过 KKL 线使用。**车辆** 页面会告诉你某辆车适合哪种方式。欧 5 起的摩托车，用一根适配其诊断插头的转接线就能用。

## 在我的车上测试过吗？

说实话：还没有。Caruspex 是用模拟的控制单元测试的；在真实硬件上，到目前为止只检查过一个带 STN2232 芯片的 OBDLink，而且没有接车。请用[测试报告](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml)帮帮我们。

## Caruspex 会弄坏我的车吗？

Caruspex 只读取。它唯一写入的操作是清除故障码，而且清除前会提醒你。它从不编码，从不刷写，也从不修改匹配值。开车时请不要使用。

## 它能改里程、防盗器或者编码功能吗？

不能，以后也不会。这在法律上很敏感，也不是这个工具的用途。

## 为什么适配器显示 2.1 版，有些功能却不行？

很多便宜的适配器报出的 ELM327 版本比实际的要新。**适配器测试** 会查明适配器真正支持哪些命令。

## 为什么我的老车数据这么慢？

走 K-Line 的车（在欧洲大约是 2001 到 2007 年）本来应答就慢。CAN 总线的车要快得多。

## Caruspex 会把我的数据发到别处吗？

不会。只有更新检查会向 GitHub 询问最新版本，而且可以关掉。见[隐私](privacy.md)。

## 我能帮什么忙？

发一份你车上的测试报告，分享一段记录，报告错误，或者改进翻译：指南是 [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md)，设置里的 **改进翻译** 按钮也会打开它。

也欢迎提交代码：[CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md)（英文）说明了如何构建 Caruspex 以及 pull request 需要满足什么。
