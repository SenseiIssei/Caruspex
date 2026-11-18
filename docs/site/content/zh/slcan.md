# SLCAN（Lawicel）

SLCAN 是一种通过串口给 CAN 棒使用的简单文本协议：刷了 slcan 固件的 CANable、USBtin、Lawicel CANUSB 以及很多自制设备。它可以在 Windows、Linux 和 macOS 上使用。

## 连接

1. 把 CAN 棒连到电脑，并用一根线连到 OBD 诊断座：CAN high 接第 6 针，CAN low 接第 14 针，地线接第 4 和第 5 针。
2. 在 **连接** 页面选择 **CAN 接口**。SLCAN 棒会被自动端口搜索找到，并出现在列表中。
3. 选择比特率（CAN 上的 OBD 用 500 kbit/s），然后连接。

## 能做什么

和所有原始 CAN 接口一样：OBD、通过 UDS 访问每一个控制单元、并行搜索控制单元、VW TP 2.0 以及监听广播数据。通过 SLCAN 无法使用 CAN FD。

## 小贴士

- 刷了 candleLight 固件的 CANable 不是 SLCAN 设备：在 Linux 上它显示为 [SocketCAN](socketcan.md) 接口。
- CAN 棒在 OBD 诊断座上不需要终端电阻，车上自己有。

## 命令行

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
