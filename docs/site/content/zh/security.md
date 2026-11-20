# 安全与更新

本页是 Caruspex 的安全信息：程序的用途、需要了解的风险、更新的方式以及如何卸载。内容依照欧盟《网络韧性法案》（Cyber Resilience Act，附件 II）要求提供给用户的信息，只包括适用于免费、非商业程序的部分。

## 程序

- **名称**：Caruspex，包括应用 `caruspex` 和命令行工具 `caruspex_cli`，支持 Windows、Linux 和 macOS。版本号写在每个下载文件的名称中，也写在每个版本的更新列表里。
- **发布者**：[GitHub](https://github.com/SenseiIssei/Caruspex) 上的 Caruspex 项目，以 MIT 许可证发布的自由软件。问题和错误请提交到 [issues](https://github.com/SenseiIssei/Caruspex/issues)。
- **用途**：用你自己的适配器，在你自己的电脑上读取并解释汽车、摩托车和卡车的诊断数据。没有账户，没有云，没有跟踪（[隐私](privacy.md)）。
- **安全特性**：读取绝不会改变车上的任何东西，凡是会让车辆执行动作的操作都会先询问。只有签名与应用内置密钥相符的更新才会被安装。Caruspex 不打开任何网络端口，也不执行来自文件或互联网的代码。

## 报告漏洞 {#report}

请通过[新建安全公告的表单](https://github.com/SenseiIssei/Caruspex/security/advisories/new)私下报告安全问题，不要发到公开的 issue。后续流程见[安全策略](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md)。已修复的漏洞会以[安全公告](https://github.com/SenseiIssei/Caruspex/security/advisories)的形式公布。

## 需要了解的风险 {#risks}

- 只从[发布页面](https://github.com/SenseiIssei/Caruspex/releases)下载 Caruspex。来自其他地方的副本可能被改动过。
- 只打开来自可信之人的记录、PID 包和 3D 模型。它们无法执行代码，但损坏的文件可能让 Caruspex 变慢或关闭。
- 只在车辆静止、位于安全地点时清除故障码、进行执行器测试和基本设定。
- 许多蓝牙和 Wi-Fi 适配器没有 PIN 码或使用开放网络，附近任何人都能连接。用完后请拔下适配器。
- ENET 线、DoIP 和 Wi-Fi 适配器不做任何加密：只能直接连接或在自己的网络中使用。
- J2534 驱动是其制造商的程序，并以你的权限运行。只从制造商处安装。

## 更新与支持 {#support}

- 每次安装时，包括从旧版本更新之后，Caruspex 都会问一次“要检查更新吗？”。只有回答“是”之后，它才会在启动时以及运行期间每六小时向 GitHub 询问是否有新版本。新版本只会点亮标题栏里的一个按钮，从不打断你。安装只需点一下，且只安装签名相符的软件包。AppImage 和 macOS 应用会自行更新；.deb 软件包和 Android 应用会得到下载页面。
- 检查更新和自动安装是设置中 **窗口与语言** 下的开关，可以随时更改。没有回答“是”，就不会向 GitHub 发送任何请求；这时请不时查看发布页面。
- 安全修复只进入最新版本。旧版本不会单独获得修复，所以请保持 Caruspex 为最新。
- Caruspex 由一个人在业余时间维护，只在时间允许的范围内进行，不作任何承诺：没有支持期限，也没有响应时间。如果停止维护，代码仓库会被归档并注明这一点。

## 校验下载 {#verify}

每个版本都附有 `SHA256SUMS` 文件，列出每个文件的校验和，并附有 GitHub 签名的构建来源记录。把文件放在同一个文件夹中：

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

没有这些工具时，可以在 PowerShell 中用 `Get-FileHash <文件>`（Windows）或 `shasum -a 256 <文件>`（macOS）显示校验和，再与 `SHA256SUMS` 中的那一行比较。

## 卸载 Caruspex 并删除数据 {#remove}

1. 卸载程序：Windows 在“设置”的“应用”中；macOS 把 Caruspex 从“应用程序”中移走；Linux 删除 AppImage 或运行 `sudo apt remove caruspex`。
2. 删除数据：设置中的 **打开存储位置** 按钮会显示存放设置、车库和崩溃报告的文件夹。删除这些文件夹。你自己保存的记录和报告在你保存的位置。

## 面向开发者和企业 {#integrators}

- 每个版本都为每个平台附有软件物料清单 `Caruspex-<version>-sbom-<platform>.cdx.json`（CycloneDX），列出每个库及其版本。
- 关于 Caruspex 的风险评估、要求清单以及《网络韧性法案》的适用范围，见 [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra)。
- Caruspex 并未作为《网络韧性法案》意义上的产品投放市场，因此没有欧盟符合性声明，也没有 CE 标志。
