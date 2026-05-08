<div align="center">
  <img src="docs/logo.svg" width="140" alt="yande-dl logo" />

  <h1>yande-dl</h1>

  <p>
    现代、轻量的图板订阅下载工具。<br/>
    无数据库、无格式锁定,只用文件夹。
  </p>

  <p>
    <a href="README.md">English</a> ·
    <a href="README.zh-TW.md">繁體中文</a> ·
    <strong>简体中文</strong>
  </p>
</div>

---

> 大多数 booru 下载器要么 UI 像 Windows 95,要么想当你的图库管理器。
> yande-dl 两者都不是——它只做三件事:订阅标签、批量下载所有匹配的图、然后别挡你路。
> 文件放在你选的文件夹里,任何文件管理器都能浏览。

## 特性

- **按标签批量下载** — 抓取整个 tag 的所有 post,自动翻页,按文件名去重。
- **增量更新带失败重试保护** — 重跑订阅只抓比上次新的;失败的图会在下次自动重试,绝不静默跳过。
- **支持多个图板** — Yande.re 与 Konachan;架构预留扩展。
- **多语言界面** — English、繁體中文、简体中文。自动检测系统语言,可在「设置」切换。
- **导入/导出** — 订阅列表就是一个 `tags.json` 文件。
- **默认礼貌客户端** — 默认 3 并发、300ms 最小间隔、可识别 User-Agent、默认只下 safe。
- **现代化 UI** — Tauri 2 + React,深色默认。
- **本地优先、零 DB** — JSON 配置 + 文件夹扫描,无 SQLite、无遥测。

## 安装

到 [Releases](https://github.com/KenDev099/yande-dl/releases) 下载对应版本:

- macOS:`yande-dl_<ver>_aarch64.dmg` / `_x64.dmg`
- Windows:`yande-dl_<ver>_x64-setup.exe`
- Linux:`yande-dl_<ver>_amd64.deb` / `.AppImage`

> **v0.1 为未签名版本** — macOS 首次启动需在「系统设置 → 隐私与安全性」点击允许。
> 代码签名将于 v0.2 启用。

### 从源码构建

```bash
git clone https://github.com/KenDev099/yande-dl
cd yande-dl

# 安装前端依赖(同时会带入 Tauri CLI,需提前安装 Tauri 2 系统依赖
# 参见 https://tauri.app/start/prerequisites/)。
pnpm install --dir ui

# 从项目根目录运行 — Tauri CLI 需要找到 crates/yande-dl-tauri/tauri.conf.json。
pnpm dev          # tauri dev(Rust + Vite 实时刷新)
pnpm build        # tauri build(产出正式版)
```

要求:Rust 1.75+、Node 20+、pnpm 9+、各平台 Tauri 系统依赖。

## 使用方法

1. 启动后完成首次设置 modal(下载文件夹、默认分级、年龄确认)。
2. 在「订阅」页新建 tag(例如 Yande.re 的 `stella_sora`)。
3. 点击「下载」 — yande-dl 保存到 `<root>/_yande stella_sora/yande_<post_id>.<ext>`。
4. 用系统文件管理器浏览。yande-dl 故意不做图库浏览器。

第一次完成之后,「更新」按钮只抓比上次新的图,并额外翻 2 页以补回先前临时失败的图。

## 配置文件位置

配置文件保存在系统的 app data 文件夹中:

- macOS:`~/Library/Application Support/yande-dl/`
- Windows:`%APPDATA%\yande-dl\`
- Linux:`~/.config/yande-dl/`

高级用户可直接编辑 `tags.json` 与 `settings.json`,原子写入保证不会损坏。
如需详细日志,设置环境变量 `KURA_LOG=debug`。

## 架构(一段话)

Rust 端分四个 crate:**`yande-dl-core`** 负责数据模型、`ImageProvider` trait、
sanitize 与 retry helper、`Downloader`(文件夹扫描去重、MD5 验证、可取消)、
以及 `JobRunner`(增量更新 lookback、失败保护的安全 baseline)。
**`yande-dl-providers`** 实现 `MoebooruProvider`,Yande.re 与 Konachan 共用。
**`yande-dl-config`** 持久化 `tags.json` 与 `settings.json`,含原子写入与损坏恢复。
**`yande-dl-tauri`** 是装配层 — Tauri 2 + commands + events。
前端是 React 18 + Tailwind + shadcn 风组件,通过 TanStack Query 与 typed IPC 串接。

## ⚖️ 法律与责任声明

yande-dl 是**客户端工具**。它不托管、不分发、不生成任何图片内容。
所有图片都是直接从第三方网站下载到用户本机。

**用户须自行承担以下责任:**

- 确认自己符合所在司法管辖区的合法年龄。
- 遵守来源网站的服务条款。
- 尊重个别图片的著作权(图板上多为用户上传的同人作品;原作者保留版权,
  请直接支持原作者)。

**yande-dl 被设计为礼貌的客户端:**

- 保守的速率限制(默认 3 并发、最小间隔 300ms)。
- 尊重 `Retry-After` header。
- 可识别的 User-Agent:`yande-dl/<version> (+https://github.com/KenDev099/yande-dl)`。
- 默认 `safe` 过滤;成人内容需要明确启用。

## 开发路线图

- [x] **v0.1.0** — Yande.re + Konachan、订阅、增量更新、导入/导出、多语言
- [ ] **v0.2** — 多 tag 搜索、「下载前预览」、JPG 模式、command palette、自动更新
- [ ] **v1.0** — Danbooru、Gelbooru、e621、Pool 支持、CLI 模式

## 许可证

[MIT](LICENSE) © 2026

## 致谢

- [Yande.re](https://yande.re)、[Konachan](https://konachan.com) 以及整个 Moebooru 社群。
- [Tauri](https://tauri.app)、[shadcn/ui](https://ui.shadcn.com)、[TanStack](https://tanstack.com)、[Radix UI](https://radix-ui.com)。
- 所有贡献作品到图板上的画师。
