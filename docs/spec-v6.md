# yande-dl 技術規劃文件 v6
> 跨平台圖板批次下載與訂閱管理工具
> v0.1 → v1.0 完整實作藍圖（無 DB、修正版）
> **v5 變更**：專案更名為 yande-dl（Yande + dl，與 yt-dlp 同一命名慣例）。
>
> **v3 與 v2 差異**：修正 MD5/JPEG 衝突、補上取消機制與重試邏輯、修正增量更新終止 bug、加入檔名 sanitize 與 tag normalization、簡化前端、把 v0.1 切成 alpha/beta/1.0 三段。詳見「決策紀錄」段落。
---
## 目錄
1. [專案總覽](#1-專案總覽)
2. [架構總覽與決策紀錄](#2-架構總覽與決策紀錄)
3. [目錄結構](#3-目錄結構)
4. [Rust 核心模組設計](#4-rust-核心模組設計)
5. [JSON 設定檔規格](#5-json-設定檔規格)
6. [Tauri IPC 介面](#6-tauri-ipc-介面)
7. [前端結構](#7-前端結構)
8. [下載流程 Sequence Diagram](#8-下載流程-sequence-diagram)
9. [錯誤處理與韌性策略](#9-錯誤處理與韌性策略)
10. [法務與倫理](#10-法務與倫理)
11. [開發里程碑](#11-開發里程碑)
12. [CI/CD](#12-cicd)
13. [README 模板](#13-readme-模板)
14. [風險與權衡](#14-風險與權衡)
15. [分階段實作 Prompt 範本](#15-分階段實作-prompt-範本)
---
## 1. 專案總覽
**yande-dl** 是一個用 Rust + Tauri 2.x 打造的桌面工具，定位是「給設計師與開發者用的圖板訂閱下載器」。yande-dl **不是圖庫管理器**——下載完的檔案交給 OS 檔案總管瀏覽，yande-dl 只負責「訂閱 tag、批次下載、避免重複」這三件事做到極致。
首版同時支援 Yande.re 與 Konachan（共用 Moebooru API），架構預留多 Provider 擴充。
### 核心設計原則
1. **檔案系統就是事實（Filesystem as truth）**：去重、增量更新都靠掃描下載資料夾，不維護任何 DB
2. **零鎖定（Zero lock-in）**：所有設定與訂閱都是純 JSON，使用者可手編、版控、輕鬆遷移
3. **窄而精**：v0.1 只做訂閱+下載+設定三件事，每件事做到位
4. **正確性勝過速度**：不抄捷徑換取看起來更快的下載——下載失敗、MD5 不符、取消請求都要能正確處理
### 技術棧
| 層 | 選擇 |
|----|------|
| 框架 | Tauri 2.x |
| 後端 | Rust（reqwest、tokio、tokio-util、serde、async-trait、tracing、anyhow、thiserror、md-5、uuid） |
| 前端 | React 18 + TypeScript + Vite |
| 樣式 | Tailwind CSS + shadcn/ui |
| 狀態 | TanStack Query（server state）+ React state（local UI state） |
| Toast | sonner |
| 圖標 | Lucide |
| 字體 | Inter + JetBrains Mono |
| 持久化 | **JSON 檔案**（無 DB） |
| 打包/發布 | GitHub Actions + Tauri updater |
> v2 的 Zustand、Framer Motion、cmdk 都從 v0.1 拿掉。Zustand 的工作 TanStack Query + React state 已經涵蓋；Framer Motion 對下載器這種工具型 UI 收益太低；command palette 留到 v0.2 再加。
---
## 2. 架構總覽與決策紀錄

(完整架構圖、決策紀錄與分層理由原文長度過大，未直接內嵌。詳細內容請參考對話原始 v6 訊息。)

決策紀錄摘要（D1–D12）：
- D1 Tauri vs Wails/Electron：二進位最小、Rust 後端對 IO/HTTP 強
- D2 React vs Solid/Svelte：生態最完整
- D3 無 DB，純 JSON + folder scan
- D4 4 個 crate（core / providers / config / tauri）
- D5 Yande.re 與 Konachan 共用 Moebooru adapter
- D6 v0.1 只下載 original variant（md5 對應 original）
- D7 下載完 MD5 驗證後即丟，不持久化 metadata
- D8 增量更新「回看 2 頁」防止瞬時失敗
- D9 tag 一律 normalize 成 lowercase
- D10 資料夾與檔名 sanitize（Windows 不接受字元）
- D11 TanStack Query + React state，無 Zustand
- D12 MIT 授權

---
## 3. 目錄結構

```
yande-dl/
├── Cargo.toml                       # workspace
├── crates/
│   ├── yande-dl-core/
│   │   └── src/{lib,model,provider,downloader,job,retry,sanitize,error}.rs
│   ├── yande-dl-providers/
│   │   └── src/{lib,moebooru}.rs
│   ├── yande-dl-config/
│   │   └── src/{lib,paths,tags,settings,atomic_write}.rs
│   └── yande-dl-tauri/
│       ├── tauri.conf.json
│       ├── icons/
│       └── src/{main,state,http,events,setup}.rs + commands/
└── ui/
    ├── package.json / vite.config.ts / tailwind.config.ts / index.html
    └── src/{main,App,routes}.tsx + pages/, components/, hooks/, ipc/, lib/, styles/
```

**App Data 路徑**：
- macOS: `~/Library/Application Support/yande-dl/`
- Windows: `%APPDATA%\yande-dl\`
- Linux: `~/.config/yande-dl/`

實作用 `directories` crate（`ProjectDirs::from("dev", "kura", "yande-dl")`）。

---
## 4. Rust 核心模組設計（要點）

### 4.1 模型與 trait
- `Rating`：Safe/Questionable/Explicit；`from_moebooru_code("s"|"q"|"e")`、`to_short()`
- `ImageVariant { url, width, height, size_bytes, mime }`
- `PostVariants { original, preview, sample?, jpeg? }`（v0.1 只用 original）
- `Post { provider_id, post_id, md5, rating, score, w, h, tags, artist?, source_url?, created_at?, variants, extra: HashMap }`
- `SearchQuery { tags, min_score, min_w, min_h, ratings, limit }`
- `Capabilities { max_results_per_page, uses_md5, default_sort_desc_by_id }`
- `ImageProvider`：`id()`、`display_name()`、`capabilities()`、`async fn search(query, page) -> Result<Vec<Post>, CoreError>`

### 4.2 Sanitize
- `normalize_tag(s) = s.trim().to_lowercase()`
- `safe_folder_segment(t)` 規則：
  1. 替換 `< > : " / \ | ? *` 與控制字元為 `_`
  2. 連續空白折疊為單一 `_`
  3. 開頭/結尾的 `.` 或空白移除
  4. 長度截斷 120 字元（unicode-safe）
  5. 空字串 fallback `_`

### 4.3 重試
- `RetryPolicy::standard()`：3 次、base 2000ms、max 30000ms
- `RetryPolicy::fast()`：2 次、base/max 1000ms
- `with_backoff(policy, op)`：
  - `Cancelled`、`Md5Mismatch`、`Parse` 不重試
  - `RateLimited` 尊重 `Retry-After`
  - 其他用指數退避 `base * 2^(attempt-1)`，上限 max

### 4.4 Moebooru adapter
- `MoebooruProvider::yandere(client)`、`konachan(client)` 兩個工廠
- `build_tag_string`：把 ratings、min_score/w/h 串成 Moebooru tag query
- `fetch_page` 用 `with_backoff` 包裝；HTTP 429 解析 `Retry-After`
- `normalize`：把 raw response → `Post`，缺 `jpeg_url`/`sample_url` 時對應欄位為 None

### 4.5 Downloader（folder-scan 去重，僅 original）
- 不變式：`<root>/_<provider> <safe_tag>/<provider>_<post_id>.<ext>`，副檔名不參與去重
- `scan_existing_post_ids(folder, provider_id) -> HashSet<i64>`：開工前一次性掃描
- `download_post`：
  - 已存在 → SkippedDuplicate
  - 取消 → Cancelled（用 `tokio::select!` 中斷正在進行的 body read）
  - 下載 + MD5 驗證 + `.part` → rename 原子寫入
  - 網路錯重試（standard），MD5 mismatch 再試一次（fast policy）

### 4.6 JobRunner
- 增量更新「lookback」：整頁都 ≤ baseline 時不立刻 break，再翻 N 頁（預設 2）以補上之前失敗的張數
- 黑名單比對與 baseline 終止條件分開判斷（避免黑名單誤觸發終止）
- `compute_safe_baseline`：成功項中、小於最低失敗 post_id 的最大值；無失敗則全部成功的最大值；空集合 → 沿用原 baseline

---
## 5. JSON 設定檔規格

### tags.json
```jsonc
{
  "version": 1,
  "subscriptions": [
    {
      "id": "<uuid>",
      "provider": "yande" | "konachan",
      "tag": "Stella_Sora",          // 原始輸入（顯示用）
      "normalizedTag": "stella_sora", // 用於資料夾與比對
      "lastRunAt": 1730000000,
      "lastSeenPostId": 1255110,
      "totalDownloaded": 42,
      "createdAt": 1700000000
    }
  ]
}
```

### settings.json
```jsonc
{
  "version": 1,
  "downloadRoot": "/Users/x/Pictures/yande-dl",
  "concurrency": 3,
  "minDelayMs": 300,
  "defaultRatings": ["safe"],          // 子集：safe/questionable/explicit
  "theme": "dark" | "light" | "system",
  "ageConfirmed": true,
  "blacklist": ["loli", "shota"]
}
```

關鍵設計：
- 所有 IPC 用到的 struct 一律 `#[serde(rename_all = "camelCase")]`
- 原子寫入：tmp + rename
- 損毀恢復：parse 失敗 → 備份 `tags.json.broken.<ts>` → 重建空檔 + emit error
- `TagsStore::add`：同 `provider + normalizedTag` 視為重複（不新增）

---
## 6. Tauri IPC 介面

### Commands
- `list_subscriptions() -> SubscriptionDto[]`
- `add_subscription(provider, tag) -> SubscriptionDto`
- `remove_subscription(id)`
- `export_subscriptions(dest)`、`import_subscriptions(source, mode: "replace"|"merge") -> ImportReport`
- `get_settings() -> Settings`、`update_settings(settings) -> Settings`
- `start_download(subscriptionId, incremental: bool) -> { jobId }`
- `cancel_job(jobId)`
- `list_active_jobs() -> ActiveJobDto[]`
- `open_folder(path?)`、`open_post_url(provider, postId)`

### Events（皆 camelCase payload）
- `download:progress`：`{ jobId, subscriptionId, currentPage, fetched, saved, skipped, failed, cancelled }`
- `download:completed`：`{ jobId, subscriptionId, totalSaved, totalSkipped, totalFailed, totalCancelled, safeLastPostId }`
- `notification`：`{ kind: "info"|"success"|"warning"|"error", message }`

### mpsc 設計
- progress 用 `mpsc::channel(16)` + `try_send`（滿了丟舊）
- completed 走獨立通道避免被丟

---
## 7. 前端結構

### 路由
- `/subscriptions`（主頁，含 ActiveJobsDrawer）
- `/settings`
- `FirstRunGate`：當 `settings.ageConfirmed === false || downloadRoot === null` 時彈出單一 modal

### 元件樹（簡）
```
<App><RouterProvider><FirstRunGate>
  <FirstRunModal />            // 條件
  <AppLayout>
    <Sidebar />
    <main><Outlet /></main>
    <ActiveJobsDrawer />       // 全域抽屜
    <Toaster />                 // sonner
```

### 設計 token（globals.css）
- 暗色預設、青綠 accent（HSL 180 80% 55%）
- Inter 字體，JetBrains Mono for code
- macOS vibrancy / Windows mica（在 `tauri.conf.json` 設定）

---
## 8. 下載流程 Sequence

1. 使用者點下載 → `start_download(subId, true)`
2. AppState 載入 subscription、產生 job_id + CancellationToken、插入 `active_jobs`、回傳 jobId
3. 背景 spawn JobRunner：
   - `scan_existing_post_ids` 一次（記憶體 HashSet）
   - 迴圈每頁：cancel check → provider.search（含 backoff）→ 計算 `any_above_baseline` → 過濾 baseline + 黑名單 → 並行 download_post（每張 `tokio::select!` cancel）
   - 每頁結束 emit `download:progress`（try_send）
4. 結束時 `compute_safe_baseline` → `TagsStore::update_after_run` → emit `download:completed`

---
## 9. 錯誤處理與韌性策略

| 情境 | 處理 |
|------|------|
| 網路逾時 | reqwest connect_timeout 10s + timeout 60s + standard 退避（3 次） |
| HTTP 429 | 解析 Retry-After 等待後重試 |
| HTTP 5xx | standard policy 重試後標 failed |
| 單張 MD5 不符 | fast policy 再試一次 |
| 磁碟寫入失敗 | 任務中止 + emit notification(error) |
| 取消任務 | CancellationToken → tokio::select! 中斷下載 future；`.part` 留在原地由下次任務清理 |
| .part 殘留 | scan 時刪除 24h 以上、size < 100KB 的 `.part` |
| 程式中關閉 | rename 原子化保證不留半截正式檔；in-memory active_jobs 直接消失 |
| 重複任務 | 同 subscriptionId 已有 active job → reject + warning |
| tags.json 損毀 | 備份 + 重建空檔 + error notification |
| 失敗保護 | safe baseline 不跳過失敗 post_id |

---
## 10. 法務與倫理

### First-run modal 文案
> 歡迎使用 yande-dl
> yande-dl 是 Yande.re / Konachan 的批次下載工具。圖板上的內容由使用者貢獻，包含 SFW（safe）與 NSFW（questionable / explicit）。
> yande-dl 預設只下載 safe 內容。若您未滿 18 歲、或所在地區法律禁止存取成人內容，請維持預設設定。
> [下載資料夾選擇 / 預設 rating / 年齡確認 checkbox]

### README 法律段落
- 客戶端工具，不託管/分發/生成內容
- 使用者自負法律與 ToS 責任
- 政策性 polite client：保守 rate limit、Retry-After honor、可識別 User-Agent、預設 safe

### 授權
MIT。

---
## 11. 開發里程碑

### v0.1-alpha — 「能跑通」
- Yande.re only、單 tag 訂閱、新增/刪除、全量下載、folder scan dedup、基本 settings、First-run modal、最小 UI、ActiveJobsDrawer
- 驗收：`stella_sora` 跑完整流程；重跑不重複；可取消

### v0.1-beta — 「實用」
- + Konachan provider、增量更新 + lookback、黑名單、Rating filter、Import/Export、統一 notification

### v0.1.0 — 「可發佈」
- 完整測試、三平台 CI/release、README/LICENSE/CONTRIBUTING、logo + demo.gif、tracing 接入

### 測試矩陣（必備）
- `Rating::from_moebooru_code` 邊界
- `safe_folder_segment` 各規則
- `normalize_tag`
- `compute_safe_baseline` 全/部分/首張失敗/空集合
- `MoebooruProvider::normalize` 真實 fixture / 缺 jpeg_url / 429
- `Downloader::scan_existing_post_ids` 含非預期格式
- `Downloader::download_post` 取消 / MD5 mismatch
- `JobRunner` lookback / 黑名單不誤判終止
- `TagsStore` 損毀恢復 / import-export
- `atomic_write_json` 寫到一半中斷

### 後續 Roadmap
- v0.2：多 tag 搜尋、JPG variant（不 MD5）、預覽、command palette、暫停/續傳、自訂檔名 template、i18n、auto-update
- v1.0+：Danbooru/Gelbooru/e621、Pool、CLI、感知雜湊、雲端同步

---
## 12. CI/CD

`.github/workflows/ci.yml`：
- Rust：fmt、clippy（-D warnings）、test
- 前端：pnpm install、lint、typecheck、build

`.github/workflows/release.yml`：
- 觸發：`push tags v*`
- Matrix：macOS aarch64、macOS x86_64、Linux x86_64、Windows x86_64
- Linux 補 webkit2gtk-4.1、libgtk-3、ayatana-appindicator3、librsvg2 等套件
- 用 `tauri-apps/tauri-action@v0`，產 draft release

Code signing 與 auto-updater 簽署留到 v0.2。

---
## 13. README 模板（精簡版）

- Why yande-dl：subscribe / download / stay out of your way
- Features：tag-based bulk、incremental w/ retry-aware baselines、multi-platform support、import/export、polite by design、modern UI、local-first
- Installation：DMG / setup.exe / .deb / .AppImage；build from source
- Usage：4 步驟說明
- Configuration：app data 路徑、power user 直接編 JSON
- Legal Notice：客戶端、polite、使用者自負責任
- Roadmap、License（MIT）、Acknowledgements

---
## 14. 風險與權衡

- **無 DB vs SQLite**：訂閱 < 1000、寫入頻率低、folder scan 解掉去重需求 → JSON 完美契合
- **Folder scan vs 持久化索引**：10k 張 read_dir < 50ms；正確性勝過微秒級效能
- **Tauri vs Wails/Electron**：二進位最小、Rust IO 強
- **共用 Moebooru adapter**：Yande/Konachan schema 完全相同，獨立實作 = 複製貼上
- **React vs Solid/Svelte**：生態最完整
- **v0.1 只下 original**：md5 對應 original，下 jpeg 必失敗；v0.2 加 jpg 模式時明確標註不做 MD5
- **lookback = 2 頁**：直覺值，不開放設定避免旋鈕氾濫

---
## 15. 分階段實作 Prompt 範本

七階段（對應 v0.1-alpha → v0.1-beta → v0.1.0）：

### Stage 1：Workspace + yande-dl-core 基礎
建立 workspace、4 個 crate 骨架；core 完成 model.rs、provider.rs、error.rs、sanitize.rs、retry.rs；對 `Rating::from_moebooru_code`、`normalize_tag`、`safe_folder_segment`、`with_backoff` 寫單元測試；`cargo build && cargo test --all` 全綠。

### Stage 2：Moebooru Adapter
完成 `crates/yande-dl-providers/src/moebooru.rs`；wiremock 整合測試 + fixture（真實 yande.re 回應的精簡版）；至少測：正常解析、缺 jpeg_url、HTTP 429 帶 Retry-After 後成功、HTTP 500 連續 3 次後失敗。

### Stage 3：yande-dl-config（JSON 持久化）
完成 paths.rs、atomic_write.rs、tags.rs、settings.rs；測試：add → load → remove → import/export 全流程；同 normalizedTag 視為重複；`load_or_recover()` 對損毀 JSON 自動備份重建；用 tempfile 模擬 app data 目錄。

### Stage 4：Downloader + JobRunner
完成 downloader.rs、job.rs；加入 tokio-util；測試：scan_existing_post_ids（含非預期格式）、download_post（成功/MD5 mismatch/取消）、compute_safe_baseline、run_job 整合（含 lookback 與黑名單不誤判終止）。

### Stage 5：Tauri 殼 + 前端 v0.1-alpha
建立 yande-dl-tauri 與 ui/ 骨架，目標 v0.1-alpha：tauri.conf.json、AppState、setup hook、所有 commands、events；前端 Vite + React + TS + Tailwind + shadcn/ui；FirstRunGate + AppLayout + Subscriptions（最小）+ Settings（最小）+ ActiveJobsDrawer；DTO 一律 camelCase。

### Stage 6：v0.1-beta 增益
註冊 konachan provider；增量模式（since_post_id 從 lastSeenPostId）；SubscriptionCard 新增「增量/全抓」；Settings 黑名單 + BlacklistEditor；default_ratings 套到 SearchQuery；ImportExportMenu；統一 notification event。

### Stage 7：v0.1.0 發佈
補完測試矩陣；GitHub Actions（ci.yml + release.yml）；README + LICENSE + CONTRIBUTING；logo + demo.gif；接入 tracing-subscriber（env var KURA_LOG）；Linux 上手動測試 webkit2gtk-4.1；tag v0.1.0 push 後確認 release workflow 跑出三平台產物；release notes 用 keep-a-changelog 格式。
