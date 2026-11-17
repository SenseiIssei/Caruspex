# K+DCAN 线

K+DCAN 线是大约 1998 到 2017 年的 BMW 配合维修厂软件 INPA 和 ISTA 使用的常见诊断线。里面有一块 FTDI 串口芯片和一个用于 BMW D-CAN 的小转换器。很多这种线在第 7 针和第 8 针之间有一个开关或跳线，用于老车。

## Caruspex 中的现状

> 真实现状：Caruspex 目前还不支持这根线的 D-CAN 模式。通过 K-Line 的 BMW 老协议（DS2 和 BMW 自己风格的 KWP2000）已在计划中，但需要先在真车上测试。

现在你可以改用这些：

- **F 系列起的 BMW 和 Mini（大约 2008 年以后）**：[ENET 线](enet.md)可以通过以太网访问每一个控制单元，或者用普通的 [ELM327](elm327-usb.md) 通过 CAN 配合 BMW 自己的寻址方式。
- **带 CAN 的 BMW 上的标准 OBD（大约 2007 年以后）**：任何 ELM327 都能读取发动机故障和实时数据。
- **在 K-Line 模式下**，这根线会像 [KKL 线](kkl.md)一样把 K-Line 直接透传。这对 VW 集团的车有用；K-Line 上的 BMW 控制单元需要的协议目前还缺。

## 需要你的帮助

如果你有 K+DCAN 线和一辆老款 BMW，一份附带记录的[测试报告](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml)会帮助我们完成这部分。
