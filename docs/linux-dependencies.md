# Linux Dependencies

TauriアプリケーションをLinux環境でビルド・実行するために必要なシステムライブラリの解説です。

## 必要なライブラリ一覧

```bash
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf
```

## 各ライブラリの詳細

### libgtk-3-dev

**役割**: GTK 3 GUIツールキットの開発ファイル

**提供機能**:
- ウィンドウ、ボタン、メニューなどのUI要素
- イベント処理システム
- テーマとスタイリングのサポート

**なぜ必要**: TauriはLinux上でGTK 3をベースにネイティブウィンドウを作成します。デスクトップ環境との統合に必須です。

### libwebkit2gtk-4.1-dev

**役割**: WebKit2ベースのWebレンダリングエンジン

**提供機能**:
- HTML/CSS/JavaScriptの解析と実行
- DOM操作とレンダリング
- ネットワーク通信とリソース管理

**なぜ必要**: Tauriアプリのフロントエンド（HTML/CSS/JS）を表示するために必須です。

**プラットフォーム別のWebView**:
| OS | WebView |
| --- | --- |
| macOS | WKWebView（システム標準） |
| Windows | WebView2（Microsoft Edge ベース） |
| Linux | WebKitGTK |

### libappindicator3-dev

**役割**: システムトレイアイコンのサポート

**提供機能**:
- タスクバー/システムトレイへのアイコン表示
- コンテキストメニューの実装
- 通知機能との統合

**なぜ必要**: バックグラウンドで動作するアプリや、トレイアイコン機能を実装する場合に必要です。Tauriのシステムトレイ機能を使用する際に必須となります。

### librsvg2-dev

**役割**: SVG（Scalable Vector Graphics）画像の処理ライブラリ

**提供機能**:
- SVG画像のパースとレンダリング
- ベクター画像のスケーリング
- アイコンのラスタライズ

**なぜ必要**: アプリアイコンやUI要素でSVGを使用する場合に必要です。高解像度ディスプレイでも鮮明なアイコン表示を実現します。

### patchelf

**役割**: ELF（Executable and Linkable Format）バイナリの修正ツール

**提供機能**:
- 実行ファイルのrpathの変更
- インタープリターパスの調整
- 動的ライブラリ依存関係の修正

**なぜ必要**: Tauriがビルドしたバイナリをポータブルにするために使用されます。配布時に動的リンクライブラリのパスを調整し、異なるLinux環境でも動作するようにします。

## なぜLinuxだけこれらが必要なのか

### プラットフォーム別のアプローチ

**macOS / Windows**:
- システム標準のWebViewが組み込まれている
- OSの更新と共にWebViewも更新される
- 追加のライブラリインストールが不要

**Linux**:
- システム標準のWebViewが存在しない
- ディストリビューションごとにパッケージ管理が異なる
- 開発者が明示的にライブラリをインストールする必要がある

### Linuxディストリビューション別のインストール方法

#### Debian / Ubuntu
```bash
sudo apt-get update
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev patchelf
```

#### Fedora / RHEL / CentOS
```bash
sudo dnf install gtk3-devel webkit2gtk3-devel \
  libappindicator-gtk3-devel librsvg2-devel
```

#### Arch Linux
```bash
sudo pacman -S gtk3 webkit2gtk libappindicator-gtk3 librsvg
```

## CI/CD環境での注意点

GitHub Actions等のCI環境では、ビルドステップの前にこれらのライブラリをインストールする必要があります。

```yaml
- name: Install Linux dependencies
  if: runner.os == 'Linux'
  run: |
    sudo apt-get update
    sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
      libappindicator3-dev librsvg2-dev patchelf
```

## トラブルシューティング

### エラー: `glib-2.0.pc` が見つからない

**症状**: ビルド時に `The file 'glib-2.0.pc' needs to be installed` というエラーが表示される

**原因**: GTK開発ファイルがインストールされていない

**解決方法**:
```bash
sudo apt-get install -y libgtk-3-dev
```

### エラー: `webkit2gtk-4.0.pc` が見つからない

**症状**: ビルド時に WebKit2 関連のエラーが表示される

**原因**: WebKit2GTK開発ファイルがインストールされていない

**解決方法**:
```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev
```

### パッケージバージョンの確認

インストールされているライブラリのバージョンを確認する方法:

```bash
# GTK バージョン確認
pkg-config --modversion gtk+-3.0

# WebKit2GTK バージョン確認
pkg-config --modversion webkit2gtk-4.0

# librsvg バージョン確認
pkg-config --modversion librsvg-2.0
```

## 参考リンク

- [Tauri Prerequisites - Linux](https://tauri.app/v1/guides/getting-started/prerequisites#linux)
- [GTK Documentation](https://docs.gtk.org/)
- [WebKitGTK](https://webkitgtk.org/)
