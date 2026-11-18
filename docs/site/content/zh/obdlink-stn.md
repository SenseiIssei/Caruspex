# OBDLink 与 STN 芯片

OBDLink 适配器（SX、EX、MX+、CX、LX）使用 OBD Solutions 的 STN 芯片。ELM327 能做的它们都能做，而且更多；Caruspex 会自己识别出它们并使用这些额外功能。

## Caruspex 用到了什么

- **批量请求**：Caruspex 一次最多请求 24 项实时数据，而不是 6 项。通过蓝牙时，这让一轮数据的速度快了将近一倍。
- **监听**：广播数据（例如 Tesla 上的）用 STN 自己的命令记录，这些命令在收到设定数量的报文后会自动停止。
- **Ford MS-CAN 和 GM Single-Wire-CAN**：Ford 和 GM 的车身控制单元不在普通的诊断 CAN 上。如果适配器带有额外的收发器，**所有控制单元** 页面就会提供“Ford：MS-CAN 上的车身单元”或“GM：Single-Wire-CAN 上的车身单元”。

| 芯片 | 适配器 | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 到 STN2232 | OBDLink EX | 是 | 否 |
| STN1150 到 1152、1170、2120、2255、2256 | OBDLink MX+ 等 | 是 | 是 |
| 其他 | OBDLink SX、LX、CX | 否 | 否 |

## 连接

OBDLink 适配器和任何 ELM327 一样连接：通过 USB（EX 和 SX）、蓝牙（MX+、LX、CX），或者按照[通过 USB 连接 ELM327](elm327-usb.md)和[通过蓝牙连接 ELM327](elm327-bluetooth.md)中的说明。

## 已测试

一个带 STN2232 芯片的 OBDLink 通过了适配器测试：芯片、固件和命令都被正确识别。它还没有在车上用过，所以批量请求、MS-CAN 和 SW-CAN 在真实硬件上仍未得到确认。
