# 安装

每个版本都在 [GitHub 上的发布页面](https://github.com/SenseiIssei/Caruspex/releases/latest)。安装包目前还没有签名，所以 Windows 和 macOS 会在首次启动前询问一次。

## Windows 10 和 11

| 文件 | 说明 |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | 安装程序。不需要管理员权限，可以把命令行工具 `caruspex_cli` 加到搜索路径里，只有在缺少 WebView2 运行库时才会安装它。 |
| `Caruspex-<version>-windows-x64.zip` | 便携版：解压后启动 `caruspex.exe`。 |

首次启动时，Windows SmartScreen 可能会提示这是未知应用：点击“更多信息”，再点“仍要运行”。

USB 适配器需要对应其 USB 芯片的驱动。Windows 10 和 11 通常会自己找到；如果没有，请看[通过 USB 连接 ELM327](elm327-usb.md)。

## macOS 13 及更新版本（Apple Silicon）

下载 `Caruspex-<version>-macos.dmg`，打开后把 Caruspex 拖到“应用程序”中。因为这个应用还没有经过公证，第一次请用右键点击并选择“打开”。

## Linux（x86_64）

| 文件 | 说明 |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | 几乎在所有发行版上都能运行：用 `chmod +x` 设为可执行后启动。它还会自己更新。 |
| `Caruspex-<version>-linux-amd64.deb` | 适用于 Debian 和 Ubuntu：`sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

要在不用 `sudo` 的情况下使用串口适配器，请把自己加入 `dialout` 组一次，然后注销并重新登录：

```
sudo usermod -aG dialout $USER
```

## 只用命令行工具

用于脚本和服务器的 `caruspex_cli` 不带界面，适用于 Windows、Linux 和 macOS。`caruspex_cli help` 会列出所有命令。

`caruspex_cli report` 把诊断报告写成 PDF 文件，不需要应用程序，并支持其他命令能用的所有连接方式（`--demo`、`--port`、`--tcp`、`--enet`、`--can`、`--replay`）：

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

报告可以是德语、英语、西班牙语、法语、葡萄牙语、意大利语、荷兰语和印尼语（`--lang`）。其他语言（包括中文）会输出为英语，因为每个 PDF 阅读器内置的字体里没有这些语言的文字。

## 更新

启动时，Caruspex 会向 GitHub 询问是否有新版本，除此之外什么都不做。只有当新版本的签名与应用内置的密钥相符时才会安装。检查和自动安装都可以在设置中关闭。

## 软件包管理器

winget、Homebrew、Flathub 和 Snap 的软件包正在准备中。在它们发布之前，请用上面的文件安装 Caruspex。
