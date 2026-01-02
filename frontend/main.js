// Tauri APIの取得（安全に）
let invoke;

// DOM要素（初期化時に取得）
let urlsTextarea;
let formatSelect;
let qualitySelect;
let downloadBtn;
let queueList;
let dependencyWarning;
let missingDeps;
let showInstallGuideBtn;
let installModal;
let closeModalBtn;
let installGuideContent;

// 状態管理
let updateInterval = null;

// 初期化
async function init() {
    // Tauri APIの確認
    if (!window.__TAURI__) {
        console.error('Tauri API is not available!');
        alert('アプリの初期化に失敗しました。Tauriランタイムが読み込まれていません。');
        return;
    }

    invoke = window.__TAURI__.tauri.invoke;
    console.log('Tauri API loaded successfully');

    // DOM要素を取得
    urlsTextarea = document.getElementById('urls');
    formatSelect = document.getElementById('format');
    qualitySelect = document.getElementById('quality');
    downloadBtn = document.getElementById('download-btn');
    queueList = document.getElementById('queue-list');
    dependencyWarning = document.getElementById('dependency-warning');
    missingDeps = document.getElementById('missing-deps');
    showInstallGuideBtn = document.getElementById('show-install-guide');
    installModal = document.getElementById('install-modal');
    closeModalBtn = document.getElementById('close-modal');
    installGuideContent = document.getElementById('install-guide-content');

    console.log('DOM elements loaded');

    await checkDependencies();
    startQueueUpdate();
    setupEventListeners();
}

// 依存関係チェック
async function checkDependencies() {
    try {
        const status = await invoke('check_deps');

        if (!status.all_installed) {
            const missing = [];
            if (!status.yt_dlp_installed) missing.push('yt-dlp');
            if (!status.ffmpeg_installed) missing.push('ffmpeg');

            missingDeps.textContent = `不足: ${missing.join(', ')}`;
            dependencyWarning.classList.remove('hidden');
            downloadBtn.disabled = true;
        }
    } catch (error) {
        console.error('依存関係チェック失敗:', error);
    }
}

// インストールガイド表示
async function showInstallGuide() {
    try {
        const guide = await invoke('get_install_guide');

        installGuideContent.innerHTML = `
            <h3>プラットフォーム: ${guide.platform}</h3>

            <h4>yt-dlpのインストール</h4>
            <pre><code>${guide.yt_dlp_command}</code></pre>

            <h4>ffmpegのインストール</h4>
            <pre><code>${guide.ffmpeg_command}</code></pre>

            <h4>注意事項</h4>
            <ul>
                ${guide.notes.map(note => `<li>${note}</li>`).join('')}
            </ul>
        `;

        installModal.classList.remove('hidden');
    } catch (error) {
        console.error('インストールガイド取得失敗:', error);
    }
}

// ダウンロード開始
async function startDownload() {
    const urls = urlsTextarea.value.trim();

    if (!urls) {
        alert('URLを入力してください');
        return;
    }

    const format = formatSelect.value;
    const quality = qualitySelect.value;

    try {
        downloadBtn.disabled = true;
        downloadBtn.textContent = '追加中...';

        const ids = await invoke('add_download', {
            urls,
            format,
            quality
        });

        console.log(`${ids.length}件のダウンロードを追加しました`);
        urlsTextarea.value = '';

        await updateQueue();
    } catch (error) {
        alert(`エラー: ${error}`);
    } finally {
        downloadBtn.disabled = false;
        downloadBtn.textContent = 'ダウンロード開始';
    }
}

// キュー更新
async function updateQueue() {
    try {
        const items = await invoke('get_queue');
        renderQueue(items);
    } catch (error) {
        console.error('キュー取得失敗:', error);
    }
}

// キュー表示
function renderQueue(items) {
    if (items.length === 0) {
        queueList.innerHTML = `
            <div class="empty-state">
                <p>ダウンロードキューは空です</p>
            </div>
        `;
        return;
    }

    queueList.innerHTML = items.map(item => createDownloadItemHTML(item)).join('');

    // キャンセルボタンのイベント設定
    items.forEach(item => {
        const cancelBtn = document.getElementById(`cancel-${item.id}`);
        if (cancelBtn) {
            cancelBtn.onclick = () => cancelDownload(item.id);
        }
    });
}

// ダウンロード項目のHTML生成
function createDownloadItemHTML(item) {
    const statusClass = `status-${item.status.toLowerCase()}`;
    const statusText = getStatusText(item.status);
    const showProgress = item.status === 'Downloading' || item.status === 'Converting';
    const canCancel = item.status === 'Queued' || item.status === 'Downloading' || item.status === 'Converting';

    return `
        <div class="download-item">
            <div class="download-header">
                <div class="download-info">
                    <h3>${item.title || item.url}</h3>
                    <div class="download-meta">
                        <span class="status-badge ${statusClass}">${statusText}</span>
                        <span> · </span>
                        <span>${item.format} (${item.quality})</span>
                    </div>
                </div>
                <div class="download-actions">
                    ${canCancel ? `<button id="cancel-${item.id}" class="btn-cancel">キャンセル</button>` : ''}
                </div>
            </div>

            ${showProgress ? `
                <div class="progress-container">
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: ${item.progress}%"></div>
                    </div>
                    <div class="progress-text">${item.progress.toFixed(1)}%</div>
                </div>
            ` : ''}

            ${item.error ? `
                <div class="error-message">
                    エラー: ${item.error}
                </div>
            ` : ''}
        </div>
    `;
}

// ステータステキスト取得
function getStatusText(status) {
    const statusMap = {
        'Queued': '待機中',
        'Downloading': 'ダウンロード中',
        'Converting': '変換中',
        'Completed': '完了',
        'Failed': '失敗',
        'Cancelled': 'キャンセル'
    };
    return statusMap[status] || status;
}

// ダウンロードキャンセル
async function cancelDownload(id) {
    try {
        await invoke('cancel_download', { id });
        await updateQueue();
    } catch (error) {
        console.error('キャンセル失敗:', error);
    }
}

// キュー自動更新開始
function startQueueUpdate() {
    updateQueue();
    updateInterval = setInterval(updateQueue, 1000);
}

// イベントリスナー設定
function setupEventListeners() {
    downloadBtn.addEventListener('click', startDownload);
    showInstallGuideBtn.addEventListener('click', showInstallGuide);
    closeModalBtn.addEventListener('click', () => {
        installModal.classList.add('hidden');
    });

    // モーダル外クリックで閉じる
    installModal.addEventListener('click', (e) => {
        if (e.target === installModal) {
            installModal.classList.add('hidden');
        }
    });
}

// Tauri APIが読み込まれるまで待機
function waitForTauri() {
    return new Promise((resolve) => {
        if (window.__TAURI__) {
            resolve();
            return;
        }

        const checkInterval = setInterval(() => {
            if (window.__TAURI__) {
                clearInterval(checkInterval);
                resolve();
            }
        }, 50);

        // 5秒経ってもダメならタイムアウト
        setTimeout(() => {
            clearInterval(checkInterval);
            if (!window.__TAURI__) {
                console.error('Tauri API failed to load within timeout');
                alert('Tauri APIの読み込みに失敗しました。アプリを再起動してください。');
            }
        }, 5000);
    });
}

// アプリ初期化
window.addEventListener('DOMContentLoaded', async () => {
    console.log('DOM loaded, waiting for Tauri API...');
    await waitForTauri();
    console.log('Tauri API ready, initializing app...');
    init();
});
