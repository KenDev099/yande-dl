<div align="center">
  <img src="docs/logo.svg" width="140" alt="yande-dl logo" />

  <h1>yande-dl</h1>

  <p>
    現代、輕量的圖板訂閱下載工具。<br/>
    無資料庫、無格式綁定,只用資料夾。
  </p>

  <p>
    <a href="README.md">English</a> ·
    <strong>繁體中文</strong> ·
    <a href="README.zh-CN.md">简体中文</a>
  </p>
</div>

---

> 大多數 booru 下載器要嘛 UI 像 Windows 95,要嘛想當你的圖庫管理器。
> yande-dl 兩者皆非——它只做三件事:訂閱標籤、批次下載所有符合的圖、然後別擋你路。
> 檔案放在你選的資料夾,任何檔案管理器都能瀏覽。

## 特色

- **以標籤為單位批次下載** — 抓取整個 tag 的所有 post,翻頁、用檔名去重。
- **增量更新含失敗重試保護** — 重跑訂閱只抓比上次新的;失敗的圖會在下次自動重試,絕不被默默跳過。
- **支援多個圖板** — Yande.re 與 Konachan;架構預留可擴充。
- **多國語系介面** — English、繁體中文、简体中文。自動偵測系統語系,可在「設定」切換。
- **匯入/匯出** — 訂閱清單就是一個 `tags.json` 檔案。
- **預設禮貌客戶端** — 預設 3 並發、300ms 最小間隔、可識別 User-Agent、預設只下 safe。
- **現代化 UI** — Tauri 2 + React,深色預設。
- **本地優先、零 DB** — JSON 設定 + 資料夾掃描,無 SQLite、無遙測。

## 安裝

至 [Releases](https://github.com/KenDev099/yande-dl/releases) 下載對應版本:

- macOS:`yande-dl_<ver>_aarch64.dmg` / `_x64.dmg`
- Windows:`yande-dl_<ver>_x64-setup.exe`
- Linux:`yande-dl_<ver>_amd64.deb` / `.AppImage`

> **v0.1 為未簽署版本** — macOS 首次開啟需在「系統設定 → 隱私權與安全性」按允許。
> 程式簽署將於 v0.2 啟用。

### 從原始碼建置

```bash
git clone https://github.com/KenDev099/yande-dl
cd yande-dl

# 安裝前端依賴(順便會帶入 Tauri CLI,須事先安裝 Tauri 2 系統依賴
# 參見 https://tauri.app/start/prerequisites/)。
pnpm install --dir ui

# 從專案根目錄執行 — Tauri CLI 需要找到 crates/yande-dl-tauri/tauri.conf.json。
pnpm dev          # tauri dev(Rust + Vite live-reload)
pnpm build        # tauri build(產出正式版)
```

需求:Rust 1.75+、Node 20+、pnpm 9+、各平台 Tauri 系統依賴。

## 使用方式

1. 啟動後完成首次設定 modal(下載資料夾、預設分級、年齡確認)。
2. 至「訂閱」頁面新增 tag(例如 Yande.re 的 `stella_sora`)。
3. 點選「下載」 — yande-dl 會存到 `<root>/_yande stella_sora/yande_<post_id>.<ext>`。
4. 用作業系統的檔案管理器瀏覽。yande-dl 故意不做圖庫瀏覽器。

第一次跑完之後,「更新」按鈕只會抓比上次新的圖,並多翻 2 頁以補回先前暫時失敗的張數。

## 設定檔位置

設定檔存在系統的 app data 資料夾:

- macOS:`~/Library/Application Support/yande-dl/`
- Windows:`%APPDATA%\yande-dl\`
- Linux:`~/.config/yande-dl/`

進階使用者可直接編輯 `tags.json` 與 `settings.json`,原子寫入確保不會壞檔。
若需詳細日誌,設環境變數 `KURA_LOG=debug`。

## 架構(一段話)

Rust 端分四個 crate:**`yande-dl-core`** 負責資料模型、`ImageProvider` trait、
sanitize 與 retry helper、`Downloader`(資料夾掃描去重、MD5 驗證、可取消)、
以及 `JobRunner`(增量更新 lookback、失敗保護的安全 baseline)。
**`yande-dl-providers`** 實作 `MoebooruProvider`,Yande.re 與 Konachan 共用。
**`yande-dl-config`** 持久化 `tags.json` 與 `settings.json`,含原子寫入與損毀恢復。
**`yande-dl-tauri`** 是組裝層 — Tauri 2 + commands + events。
前端是 React 18 + Tailwind + shadcn 風元件,以 TanStack Query 與 typed IPC 串接。

## ⚖️ 法律與責任聲明

yande-dl 是**客戶端工具**。它不託管、不散布、不生成任何圖片內容。
所有圖片都是直接從第三方網站下載到使用者本機。

**使用者全權負責:**

- 確認自己符合所在司法管轄區的合法年齡。
- 遵守來源網站的服務條款。
- 尊重個別圖片的著作權(圖板上多為使用者上傳的同人作品;原作者保有著作權,
  建議直接支持原作者)。

**yande-dl 設計為禮貌的客戶端:**

- 保守的速率限制(預設 3 並發、最小間隔 300ms)。
- 尊重 `Retry-After` header。
- 可識別的 User-Agent:`yande-dl/<version> (+https://github.com/KenDev099/yande-dl)`。
- 預設 `safe` 過濾;成人內容須明確啟用。

## 開發路線圖

- [x] **v0.1.0** — Yande.re + Konachan、訂閱、增量更新、匯入/匯出、多語系
- [ ] **v0.2** — 多 tag 搜尋、「下載前預覽」、JPG 模式、command palette、自動更新
- [ ] **v1.0** — Danbooru、Gelbooru、e621、Pool 支援、CLI 模式

## 授權

[MIT](LICENSE) © 2026

## 致謝

- [Yande.re](https://yande.re)、[Konachan](https://konachan.com) 以及整個 Moebooru 社群。
- [Tauri](https://tauri.app)、[shadcn/ui](https://ui.shadcn.com)、[TanStack](https://tanstack.com)、[Radix UI](https://radix-ui.com)。
- 所有貢獻作品到圖板上的繪師。
