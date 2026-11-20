# セキュリティとアップデート

このページは Caruspex のセキュリティ情報です。プログラムの目的、知っておくべきリスク、アップデートの仕組み、削除の方法をまとめています。EU のサイバーレジリエンス法（Cyber Resilience Act、附属書 II）が利用者向けに求める情報のうち、無料で非商用のプログラムに当てはまるものに沿っています。

## プログラムについて

- **名前**: Caruspex。アプリ `caruspex` とコマンドラインツール `caruspex_cli` があり、Windows、Linux、macOS で動きます。バージョンは各ダウンロードのファイル名と、各リリースの変更一覧に書かれています。
- **発行元**: [GitHub](https://github.com/SenseiIssei/Caruspex) 上の Caruspex プロジェクト。MIT ライセンスのフリーソフトウェアです。質問や不具合は [issues](https://github.com/SenseiIssei/Caruspex/issues) へどうぞ。
- **目的**: 自分のアダプターを使い、自分のパソコンで、自動車・バイク・トラックの診断データを読み取って分かりやすく説明します。アカウントもクラウドもトラッキングもありません（[プライバシー](privacy.md)）。
- **セキュリティ上の性質**: 読み取りで車の中が変わることはありません。車に何かをさせる操作は、必ず先に確認します。アップデートは、署名がアプリに組み込まれた鍵と一致したときだけインストールされます。Caruspex はネットワークポートを開かず、ファイルやインターネットから来たコードを実行しません。

## 脆弱性の報告 {#report}

セキュリティの問題は公開の issue ではなく、[新しいセキュリティ報告のフォーム](https://github.com/SenseiIssei/Caruspex/security/advisories/new)から非公開で報告してください。その後の流れは[セキュリティポリシー](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md)に書かれています。修正済みの脆弱性は[セキュリティアドバイザリ](https://github.com/SenseiIssei/Caruspex/security/advisories)として公開されます。

## 知っておくべきリスク {#risks}

- Caruspex は[リリースページ](https://github.com/SenseiIssei/Caruspex/releases)からだけダウンロードしてください。ほかの場所のコピーは改ざんされているかもしれません。
- 記録データ、PID パック、3D モデルは信頼できる人から受け取ったものだけを開いてください。コードを実行することはできませんが、壊れたファイルで Caruspex が遅くなったり閉じたりすることがあります。
- 故障コードの消去、アクチュエーターテスト、基本設定は、車が止まっていて安全な場所にあるときだけ行ってください。
- 多くの Bluetooth や Wi-Fi のアダプターには PIN がないか、ネットワークが開いています。近くの誰でも接続できます。作業が終わったらアダプターを抜いてください。
- ENET ケーブル、DoIP、Wi-Fi アダプターは何も暗号化しません。直接つなぐか、自分のネットワークの中だけで使ってください。
- J2534 ドライバーはそのメーカーのプログラムで、あなたの権限で動きます。メーカーから入手したものだけをインストールしてください。

## アップデートとサポート {#support}

- 起動時に Caruspex は新しいバージョンがあるか GitHub に問い合わせ、あれば表示します。インストールはワンクリックで、署名が一致するパッケージだけがインストールされます。AppImage と macOS アプリは自分で更新します。.deb パッケージと Android アプリにはダウンロードページが表示されます。
- 確認と自動インストールは、設定の **ウィンドウと言語** にあるスイッチで切り替えられます。確認を切った場合は、ときどきリリースページを見てください。
- セキュリティ修正は最新のリリースにだけ入ります。古いバージョンには個別の修正はないので、Caruspex を最新に保ってください。
- Caruspex は空き時間に保守されており、決まった終了日はありません。保守をやめる場合は、リポジトリと最後のリリースで前もって知らせます。

## ダウンロードの確認 {#verify}

各リリースには、すべてのファイルのチェックサムを載せた `SHA256SUMS` と、ファイルがどのようにビルドされたかを示す GitHub の署名付き記録があります。ファイルを同じフォルダーに置いて:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

これらのツールがない場合は、PowerShell の `Get-FileHash <ファイル>`（Windows）または `shasum -a 256 <ファイル>`（macOS）でチェックサムを表示し、`SHA256SUMS` の行と比べてください。

## Caruspex とデータの削除 {#remove}

1. プログラムをアンインストールします。Windows では設定の「アプリ」から、macOS では Caruspex をアプリケーションフォルダーから取り除き、Linux では AppImage を削除するか `sudo apt remove caruspex` を実行します。
2. データを削除します。設定の **保存場所を開く** ボタンで、設定、ガレージ、クラッシュレポートのフォルダーが表示されます。それらを削除してください。自分で保存した記録やレポートは、保存した場所にあります。

## 開発者と企業の方へ {#integrators}

- 各リリースには、プラットフォームごとのソフトウェア部品表 `Caruspex-<version>-sbom-<platform>.cdx.json`（CycloneDX）があり、すべてのライブラリとそのバージョンが載っています。
- Caruspex に関するリスク評価、要件の一覧、Cyber Resilience Act の適用範囲は [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra) にあります。
- Caruspex は Cyber Resilience Act でいう製品として市場に出されていないため、EU 適合宣言書と CE マークはありません。
