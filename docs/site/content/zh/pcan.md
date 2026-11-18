# PEAK PCAN（Windows）

PEAK-System 的 CAN 适配器（PCAN-USB、PCAN-USB FD、PCAN-USB Pro 等）可以在 Windows 上直接使用，不需要 J2534 驱动。在 Linux 上它们是 [SocketCAN](socketcan.md) 接口。

## 连接

1. 安装 PEAK 的设备驱动。它会带来库 `PCANBasic.dll`，Caruspex 会加载它。
2. 在 **连接** 页面选择 **CAN 接口**。每个后面连着设备的 PCAN 通道都会自动出现在列表中；已被其他程序占用的通道不会列出。
3. 选择比特率（CAN 上的 OBD 用 500 kbit/s），然后连接。

## 能做什么

OBD、通过 UDS 访问每一个控制单元、并行搜索控制单元、VW TP 2.0 以及监听广播数据。通过 PCAN-Basic 使用 CAN FD 目前还没有做进去。

## 命令行

```
caruspex_cli read --can pcan:USB1@500000
```
