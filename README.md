# YouTube Audio Downloader

TauriとRustで構築された、YouTubeから音声をダウンロードするデスクトップアプリケーション。

![YouTube Audio Downloader](https://img.shields.io/badge/Tauri-1.5-blue)
![Rust](https://img.shields.io/badge/Rust-1.92-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## 特徴

- 🎵 **複数フォーマット対応** - MP3, M4A, Opus, AAC, FLAC, WAV
- 🖼️ **サムネイル自動埋め込み** - 動画サムネイルをアルバムアートとして追加
- 📝 **メタデータ自動追加** - タイトル、アーティスト、日付などを自動設定
- 📊 **リアルタイム進捗表示** - ダウンロードの進行状況を視覚的に確認
- ⚡ **同時ダウンロード** - 最大3つまで同時処理
- 🎨 **ダークテーマUI** - YouTube風のモダンなデザイン
- 🔍 **依存関係チェック** - 必要なツールの自動確認とインストールガイド
- 🌐 **クロスプラットフォーム** - macOS, Windows, Linux対応

## 必要な環境

### システム要件

- **Rust** 1.70以上
- **Node.js** 16以上
- **yt-dlp** - YouTube動画のダウンロード
- **ffmpeg** - 音声変換

### インストール

#### macOS

```bash
# Homebrew経由
brew install yt-dlp ffmpeg
```

#### Windows

```bash
# winget経由
winget install yt-dlp.yt-dlp
winget install Gyan.FFmpeg
```

#### Linux (Debian/Ubuntu)

```bash
sudo apt install yt-dlp ffmpeg
```

## セットアップ

### 1. リポジトリのクローン

```bash
git clone https://github.com/glkt3912/youtube-audio-downloader.git
cd youtube-audio-downloader
```

### 2. 依存関係のインストール

```bash
npm install
```

### 3. アイコンの生成（初回のみ）

```bash
node generate-icon.js
```

## 開発

### 開発サーバーの起動

```bash
npm run dev
```

ホットリロードが有効な開発モードでアプリが起動します。

### ビルド

```bash
npm run build
```

プロダクション用のバイナリが `backend/target/release/` に生成されます。

## プロジェクト構造

```
youtube-audio-downloader/
├── backend/                  # Rustバックエンド
│   ├── src/
│   │   ├── main.rs          # エントリーポイント
│   │   ├── commands/        # Tauriコマンド
│   │   ├── services/        # ビジネスロジック
│   │   ├── models/          # データ構造
│   │   └── utils/           # ユーティリティ
│   ├── Cargo.toml           # Rust依存関係
│   ├── tauri.conf.json      # Tauri設定
│   └── build.rs             # ビルドスクリプト
├── frontend/                 # フロントエンド
│   ├── index.html           # メインUI
│   ├── styles.css           # スタイル
│   └── main.js              # ロジック
├── docs/                     # ドキュメント
│   └── backend-architecture.md
├── generate-icon.js          # アイコン生成スクリプト
└── package.json              # npm設定
```

## 使い方

1. **アプリを起動**
   ```bash
   npm run dev
   ```

2. **YouTube URLを入力**
   - 単一URL、または複数URL（改行区切り）

3. **フォーマットと品質を選択**
   - フォーマット: MP3, M4A, Opus, AAC, FLAC, WAV
   - 品質: Best, High (192kbps), Medium (128kbps), Low (96kbps)

4. **ダウンロード開始**
   - 進捗バーで状態を確認
   - 最大3つまで同時ダウンロード可能

5. **完了したファイルを確認**
   - デフォルトでは `~/Downloads` に保存
   - サムネイルが自動的にアルバムアートとして埋め込まれます
   - タイトル、アーティスト、日付などのメタデータも自動追加されます

## 技術スタック

### バックエンド

- **Tauri 1.5** - デスクトップアプリフレームワーク
- **Rust** - システムプログラミング言語
- **tokio** - 非同期ランタイム
- **serde** - シリアライゼーション
- **regex** - 正規表現

### フロントエンド

- **Vanilla JavaScript** - フレームワークレス
- **HTML5 & CSS3** - モダンなUI

### 外部ツール

- **yt-dlp** - YouTube動画ダウンロード
- **ffmpeg** - 音声・動画変換

## サムネイルとメタデータ

このアプリは、ダウンロードした音声ファイルに自動的にサムネイルとメタデータを埋め込みます。

### 埋め込まれる情報

- **サムネイル画像** - YouTubeの動画サムネイルをアルバムアートとして追加
- **タイトル** - 動画のタイトル
- **アーティスト** - チャンネル名
- **アップロード日** - 動画の公開日
- **説明** - 動画の説明文（一部）

### フォーマット別対応状況

| フォーマット | サムネイル | メタデータ | 備考 |
|------------|----------|-----------|------|
| MP3        | ✅       | ✅        | ID3タグ（APIC）で完全対応 |
| M4A        | ✅       | ✅        | MP4コンテナメタデータで対応 |
| FLAC       | ✅       | ✅        | Vorbis Commentで対応 |
| Opus       | ⚠️       | ✅        | 一部のプレイヤーで非対応 |
| AAC        | ⚠️       | ✅        | コンテナ形式に依存 |
| WAV        | ❌       | ❌        | メタデータ非対応フォーマット |

### 音楽プレイヤーでの表示

ダウンロードしたファイルを音楽プレイヤーで開くと：
- iTunes / Apple Music - アルバムアートとメタデータが表示されます
- VLC - サムネイルとタイトルが表示されます
- Windows Media Player - アルバムアートが表示されます

## トラブルシューティング

### 依存関係エラー

アプリ起動時に警告が表示される場合、yt-dlpまたはffmpegがインストールされていません。

**解決方法:**
1. アプリ内の「インストール手順を表示」ボタンをクリック
2. OSに応じたインストールコマンドを実行

### ダウンロード失敗

- YouTube URLが正しいか確認
- yt-dlpが最新版か確認: `yt-dlp --update`
- ネットワーク接続を確認

### ビルドエラー

```bash
# キャッシュをクリア
cd backend
cargo clean
cargo build
```

## 開発者向け

### ドキュメント

詳細な技術解説は [docs/backend-architecture.md](docs/backend-architecture.md) を参照してください。

### コントリビューション

1. このリポジトリをフォーク
2. 機能ブランチを作成 (`git checkout -b feature/amazing-feature`)
3. 変更をコミット (`git commit -m 'Add amazing feature'`)
4. ブランチにプッシュ (`git push origin feature/amazing-feature`)
5. プルリクエストを作成

### テスト

```bash
cd backend
cargo test
```

## ライセンス

このプロジェクトはMITライセンスの下で公開されています。詳細は [LICENSE](LICENSE) を参照してください。

## 注意事項

- このツールは教育目的で作成されています
- YouTubeの利用規約を遵守してください
- 著作権で保護されたコンテンツのダウンロードは違法です
- 個人的な使用に限定してください

## クレジット

- [Tauri](https://tauri.app/) - デスクトップアプリフレームワーク
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - YouTube動画ダウンロードツール
- [FFmpeg](https://ffmpeg.org/) - マルチメディアフレームワーク

## サポート

問題が発生した場合は、[Issues](https://github.com/glkt3912/youtube-audio-downloader/issues) で報告してください。

---

**Made with ❤️ using Rust and Tauri**
