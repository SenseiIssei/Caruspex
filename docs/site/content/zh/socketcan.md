# SocketCAN（Linux）

SocketCAN 是 Linux 内核的 CAN 层。任何有内核驱动的 CAN 适配器都可以通过它使用：PEAK PCAN-USB、Kvaser、刷了 candleLight 固件的 CANable、Raspberry Pi CAN 扩展板等等。

## 设置

用车辆的比特率启用接口（CAN 上的 OBD 用 500 kbit/s）：

```
sudo ip link set can0 up type can bitrate 500000
```

对于最新车辆上的 CAN FD，还要加上数据比特率：

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## 连接

在 **连接** 页面选择 **CAN 接口**。电脑上的每个 CAN 接口都在列表中（`can0`、`can1` 等）。选中它，选择比特率，如果是 CAN FD，就勾选 CAN FD。

## 能做什么

OBD、通过 UDS 访问每一个控制单元、并行搜索控制单元、VW TP 2.0、监听广播数据以及 CAN FD。

## 命令行

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
