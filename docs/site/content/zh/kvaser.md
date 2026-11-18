# Kvaser（Windows）

Kvaser 的 CAN 适配器（Leaf、U100、Memorator 等）可以在 Windows 上通过 Kvaser 的库 CANlib 直接使用。在 Linux 上它们是 [SocketCAN](socketcan.md) 接口。

## 连接

1. 安装 Kvaser 驱动。它会带来 `canlib32.dll`，Caruspex 会加载它。
2. 在 **连接** 页面选择 **CAN 接口**。每个后面连着设备的 Kvaser 通道都会出现在列表中；驱动的虚拟通道不会列出。
3. 选择比特率（CAN 上的 OBD 用 500 kbit/s），然后连接。

## 能做什么

OBD、通过 UDS 访问每一个控制单元、并行搜索控制单元、VW TP 2.0 以及监听广播数据。通过 CANlib 使用 CAN FD 目前还没有做进去。

## 命令行

```
caruspex_cli read --can kvaser:0@500000
```
