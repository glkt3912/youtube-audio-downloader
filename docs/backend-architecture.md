# バックエンド実装の技術解説

Rust初心者でもわかるように、YouTube Audio Downloaderのバックエンド実装を解説します。

## プロジェクト全体の構造

```
backend/
├── src/
│   ├── main.rs              # アプリのエントリーポイント
│   ├── models/              # データ構造の定義
│   │   ├── mod.rs
│   │   └── download_item.rs
│   ├── services/            # ビジネスロジック
│   │   ├── mod.rs
│   │   ├── dependency_checker.rs
│   │   ├── downloader.rs
│   │   └── queue.rs
│   ├── commands/            # フロントエンドとの通信
│   │   ├── mod.rs
│   │   ├── download.rs
│   │   └── dependencies.rs
│   └── utils/               # ユーティリティ
│       ├── mod.rs
│       └── validator.rs
├── Cargo.toml               # 依存関係の設定
├── build.rs                 # ビルド設定
└── tauri.conf.json          # Tauri設定
```

---

## 1. データモデル（models/download_item.rs）

**役割**: ダウンロード情報を保持する「設計図」

```rust
pub struct DownloadItem {
    pub id: String,              // 一意のID
    pub url: String,             // YouTubeのURL
    pub title: Option<String>,   // 動画タイトル（取得前はNone）
    pub format: AudioFormat,     // MP3, M4Aなど
    pub quality: Quality,        // Best, High, Mediumなど
    pub status: DownloadStatus,  // Queued, Downloading, Completedなど
    pub progress: f32,           // 進捗（0.0〜100.0）
    pub error: Option<String>,   // エラーメッセージ（エラー時のみ）
}
```

### Rust初心者向けポイント

- `pub`: 外部から読み書きできる公開フィールド
- `Option<T>`: 値があるかないか（Some/None）を表す
- `enum`: 限定された選択肢を定義（例：AudioFormat は Mp3, M4a, Opusのいずれか）

### enumの例

```rust
pub enum DownloadStatus {
    Queued,      // キューに入っている
    Downloading, // ダウンロード中
    Converting,  // 変換中
    Completed,   // 完了
    Failed,      // 失敗
    Cancelled,   // キャンセル
}
```

---

## 2. 依存関係チェック（services/dependency_checker.rs）

**役割**: yt-dlpとffmpegがインストールされているか確認

```rust
fn check_yt_dlp() -> bool {
    Command::new("yt-dlp")        // yt-dlpコマンドを実行
        .arg("--version")          // バージョン確認
        .output()                  // 実行結果を取得
        .map(|output| output.status.success())  // 成功したか判定
        .unwrap_or(false)          // エラーの場合はfalse
}
```

### Rust初心者向けポイント

- `Command`: 外部コマンドを実行するための型
- `.map()`: 結果を別の形に変換
- `.unwrap_or(false)`: エラー時のデフォルト値を指定

### OS別インストール手順

```rust
#[cfg(target_os = "macos")]  // macOSの場合のみコンパイル
{
    Self {
        platform: "macOS".to_string(),
        yt_dlp_command: "brew install yt-dlp".to_string(),
        // ...
    }
}
```

---

## 3. URLバリデーター（utils/validator.rs）

**役割**: YouTube URLの形式が正しいかチェック

```rust
static YOUTUBE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(https?://)?(www\.)?(youtube\.com/watch\?v=|youtu\.be/)[\w-]+").unwrap()
});
```

### Rust初心者向けポイント

- `Lazy`: 初回アクセス時に一度だけ初期化される
- `Regex`: 正規表現パターンマッチング
- `static`: プログラム全体で1つだけ存在するグローバル変数

### 対応URL形式

- `https://www.youtube.com/watch?v=dQw4w9WgXcQ`
- `https://youtu.be/dQw4w9WgXcQ`
- `youtube.com/watch?v=dQw4w9WgXcQ`

---

## 4. ダウンローダー（services/downloader.rs）

**役割**: yt-dlpを実行して音声をダウンロード

### 重要な概念: `Arc<Mutex<T>>`

```rust
pub async fn download(
    &self,
    item: Arc<Mutex<DownloadItem>>,  // 複数スレッドで共有できる安全なデータ
) -> Result<()>
```

### Arc（Atomic Reference Counter）とは？

- 複数の場所から同じデータを参照できるようにする「共有所有権」
- **比喩**: 本を複数人で読むために「貸出カード」を作るイメージ

### Mutex（Mutual Exclusion）とは？

- 同時に1つのスレッドだけがデータを変更できるようにする「ロック」
- **比喩**: トイレの鍵。1人が使っている間は他の人は待つ

### 使い方

```rust
item.lock().await.update_progress(50.0);
// 1. lock(): データへのアクセス権を取得（他のスレッドは待機）
// 2. await: 非同期でロック取得を待つ
// 3. update_progress(): データを更新
// 4. lockのスコープ終了で自動的に解放
```

### yt-dlp実行コマンド

```rust
Command::new("yt-dlp")
    .arg("--extract-audio")      // 音声のみ抽出
    .arg("--audio-format")       // フォーマット指定
    .arg("mp3")
    .arg("--audio-quality")      // 品質指定
    .arg("0")                    // 0=最高品質
    .arg("--newline")            // 進捗を行ごとに出力
    .arg(&url)
    .stdout(Stdio::piped())      // 標準出力をキャプチャ
```

### 進捗解析

```rust
let progress_regex = Regex::new(r"\[download\]\s+(\d+\.?\d*)%").unwrap();

for line in reader.lines() {
    if let Some(captures) = progress_regex.captures(&line) {
        // "[download] 45.2%" から "45.2" を抽出
        if let Ok(progress) = captures.get(1).parse::<f32>() {
            item.lock().await.update_progress(progress);
        }
    }
}
```

---

## 5. キュー管理（services/queue.rs）

**役割**: 最大3つまで同時ダウンロードを管理

### 3つのリスト

```rust
pub struct DownloadQueue {
    queue: VecDeque<...>,    // 待機中のダウンロード（先入れ先出し）
    active: Vec<...>,        // 現在実行中のダウンロード
    all_items: Vec<...>,     // 全てのダウンロード項目
}
```

### VecDequeとは？

- 両端から追加・削除できる配列
- キュー（待ち行列）の実装に最適

### 処理の流れ

```rust
pub fn start_processing(&self) {
    tokio::spawn(async move {  // 新しい非同期タスクを起動
        loop {
            // 1. アクティブな数が3未満 かつ キューが空でない
            if active.len() < 3 && !queue.is_empty() {
                // 2. キューから1つ取り出す
                let item = queue.pop_front();

                // 3. ダウンロード開始
                let handle = tokio::spawn(async move {
                    downloader.download(item).await;
                });
            }

            // 4. 500ms待機してループ
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}
```

### tokio::spawnとは？

- 非同期タスクを別のスレッドで実行
- **比喩**: 料理で「煮込んでいる間に別の作業をする」イメージ

---

## 6. Tauriコマンド（commands/）

**役割**: JavaScriptからRust関数を呼び出せるようにする

```rust
#[tauri::command]  // このマクロでJavaScriptから呼び出せる
pub async fn add_download(
    urls: String,
    format: AudioFormat,
    quality: Quality,
    queue: State<'_, Arc<DownloadQueue>>,  // アプリ全体で共有される状態
) -> Result<Vec<String>, String> {
    // URLをバリデーション
    let valid_urls = validate_urls(url_list)?;

    // キューに追加
    for url in valid_urls {
        let id = queue.add_item(url, format, quality);
        ids.push(id);
    }

    Ok(ids)  // 成功時は追加されたIDのリストを返す
}
```

### State<'_, T>とは？

- Tauriアプリ全体で共有されるグローバルな状態
- **比喩**: アプリ全体で使える「共有メモ帳」

### JavaScript側からの呼び出し

```javascript
const ids = await invoke('add_download', {
    urls: 'https://youtube.com/watch?v=xxx',
    format: 'Mp3',
    quality: 'Best'
});
```

---

## 7. main.rs（エントリーポイント）

```rust
fn main() {
    // 1. ダウンロードキューを作成（最大3同時）
    let queue = Arc::new(DownloadQueue::new(3));

    // 2. キュー処理を開始（バックグラウンドで実行）
    queue.start_processing();

    // 3. Tauriアプリをビルド
    tauri::Builder::default()
        .manage(queue)  // キューをアプリ全体で共有
        .invoke_handler(tauri::generate_handler![
            add_download,      // JavaScriptから呼び出せる関数を登録
            get_queue,
            cancel_download,
            check_deps,
            get_install_guide,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Rustの重要な概念まとめ

### 所有権（Ownership）

- Rustの最大の特徴
- 値には必ず1つの所有者がいる
- 所有者がスコープを抜けると自動的にメモリ解放

```rust
let s1 = String::from("hello");
let s2 = s1;  // s1の所有権がs2に移動（ムーブ）
// println!("{}", s1);  // エラー！s1は使えない
```

### 借用（Borrowing）

- 所有権を移動せずに参照する

```rust
let s1 = String::from("hello");
let len = calculate_length(&s1);  // &をつけて借用
println!("{}", s1);  // s1はまだ使える
```

### async/await

- 非同期処理（待っている間に他の作業ができる）

```rust
async fn download() {
    let data = fetch_data().await;  // データ取得を待つ
    process(data).await;            // 処理を待つ
}
```

---

## データの流れ

```
JavaScript (フロントエンド)
    ↓ invoke('add_download')
Tauriコマンド (commands/download.rs)
    ↓ validate_urls()
URLバリデーター (utils/validator.rs)
    ↓ queue.add_item()
ダウンロードキュー (services/queue.rs)
    ↓ 自動的に処理開始
ダウンローダー (services/downloader.rs)
    ↓ yt-dlpコマンド実行
実際のダウンロード
    ↓ 進捗更新
JavaScript (進捗バー更新)
```

---

## 参考リンク

- [Rust公式ドキュメント](https://doc.rust-lang.org/book/)
- [Tauri公式ドキュメント](https://tauri.app/)
- [tokio公式ドキュメント](https://tokio.rs/)
