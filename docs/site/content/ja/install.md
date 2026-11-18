# インストール

すべてのリリースは [GitHub のリリースページ](https://github.com/SenseiIssei/Caruspex/releases/latest)にあります。パッケージにはまだ署名がないので、Windows と macOS は初回起動の前に一度確認を求めます。

## Windows 10 と 11

| ファイル | 内容 |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | インストーラー。管理者権限は不要で、コマンドラインツール `caruspex_cli` を検索パスに追加でき、WebView2 ランタイムは足りない場合にだけインストールします。 |
| `Caruspex-<version>-windows-x64.zip` | ポータブル版：展開して `caruspex.exe` を起動します。 |

初回起動時に Windows SmartScreen が「認識されないアプリ」と表示することがあります：「詳細情報」をクリックし、次に「実行」をクリックしてください。

USB アダプターには、その USB チップ用のドライバーが必要です。Windows 10 と 11 はたいてい自分で見つけます。見つからない場合は [USB 接続の ELM327](elm327-usb.md) を参照してください。

## macOS 13 以降（Apple Silicon）

`Caruspex-<version>-macos.dmg` をダウンロードして開き、Caruspex を「アプリケーション」にドラッグします。アプリはまだ公証されていないので、初回は右クリックして「開く」で開いてください。

## Linux（x86_64）

| ファイル | 内容 |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | ほぼすべてのディストリビューションで動きます：`chmod +x` で実行可能にして起動します。自動でアップデートもします。 |
| `Caruspex-<version>-linux-amd64.deb` | Debian と Ubuntu 向け：`sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

`sudo` なしでシリアルアダプターを使うには、一度だけ自分を `dialout` グループに追加し、ログアウトしてからログインし直します：

```
sudo usermod -aG dialout $USER
```

## コマンドラインツールだけ

スクリプトやサーバー向けに、画面のない `caruspex_cli` が Windows、Linux、macOS 用にあります。`caruspex_cli help` でコマンドの一覧が出ます。

## アップデート

起動時に Caruspex は GitHub に新しいバージョンがあるかを問い合わせます。それ以外は何もしません。新しいバージョンは、その署名がアプリに組み込まれた鍵と一致した場合にだけインストールされます。確認も自動インストールも、設定でオフにできます。

## パッケージマネージャー

winget、Homebrew、Flathub、Snap 用のパッケージを準備中です。公開されるまでは、上のファイルで Caruspex をインストールしてください。
