# 适配器与接口

Caruspex 会自己判断插的是什么、车辆用的是什么协议。不管用哪种方式接入车辆，都能使用同样的功能：OBD-II、通过 UDS 访问每一个控制单元、监听广播数据、记录和回放。

| 接口 | 例子 | 页面 |
|---|---|---|
| 通过 USB 连接 ELM327 | 便宜的 ELM327 线、Vgate、Veepeak | [通过 USB 连接 ELM327](elm327-usb.md) |
| 通过蓝牙连接 ELM327 | 经典蓝牙和低功耗蓝牙插头、vLinker、Vgate iCar Pro | [通过蓝牙连接 ELM327](elm327-bluetooth.md) |
| 通过 Wi-Fi 连接 ELM327 | 自带 Wi-Fi 网络的插头 | [通过 Wi-Fi 连接 ELM327](elm327-wifi.md) |
| STN 芯片 | OBDLink SX、EX、MX+、CX | [OBDLink 与 STN](obdlink-stn.md) |
| KKL 线（VAG 409.1） | FTDI 或 CP2102 线 | [KKL 线](kkl.md) |
| K+DCAN 线 | 带开关的 BMW 诊断线 | [K+DCAN 线](kdcan.md) |
| ENET 线 | F 系列起的 BMW 和 Mini | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0、Mongoose、VCX Nano、Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable、USBtin、CANUSB | [SLCAN](slcan.md) |
| SocketCAN（Linux） | PEAK、Kvaser、带 candleLight 的 CANable、Raspberry Pi CAN 扩展板 | [SocketCAN](socketcan.md) |
| PEAK PCAN（Windows） | PCAN-USB、PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser（Windows） | Kvaser Leaf、U100 | [Kvaser](kvaser.md) |
| DoIP 线 | 大约 2016 年以后的许多车 | [DoIP](doip.md) |

## 每种能做什么？

普通的 ELM327 足以读取发动机、变速箱，以及大约 2008 年以后大多数车上通过 UDS 的其他所有控制单元。它做不了 Volkswagen 集团的老协议 KWP1281，这个协议要对每一个字节逐一确认：大约 2004 年以前的 VW、Audi、Seat 和 Skoda，需要一根 KKL 线。

原始 CAN 接口（J2534、SLCAN、SocketCAN、PCAN、Kvaser）把裸的 CAN 帧交给 Caruspex。传输协议由它自己处理，这样就可以快速并行搜索控制单元，也能用 VW TP 2.0 访问大约 2004 到 2012 年的 Volkswagen 集团车辆。

![适配器测试](screenshot-adapter.png)

## 我的适配器是正品吗？

**适配器测试** 会向适配器询问每个 ELM327 版本的命令，测量响应时间，并如实告诉你结论：很多便宜的适配器报的是 1.5 或 2.1 版，实际上只会 1.0 版的命令。Caruspex 从不向适配器写入任何东西。

> 目前已测试：一个带 STN2232 芯片的 OBDLink 通过了适配器测试，没有接车。其他所有适配器和接口都是按照它们的文档开发的，但还没有在真车上测试过。
