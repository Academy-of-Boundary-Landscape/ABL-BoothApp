# 摊盒 Booth-Kernel

<p align="center">
  <img src="docs/images/app-icon.png" width="120" alt="Booth-Kernel Logo" />
</p>

<h1 align="center">摊盒 Booth-Kernel</h1>

<p align="center">
  <b>LAN-first · Offline · Local-first 出摊系统</b><br/>
  基于 Tauri v2 + Rust 的现代化同人展会收银与库存管理工具
</p>

<p align="center">
  <a href="https://booth-kernel.secret-sealing.club">官网</a> ·
  <a href="https://booth-kernel.secret-sealing.club/guide/getting-started">使用文档</a> ·
  <a href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases">下载</a> ·
  <a href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/issues">Issue</a>
</p>

<p align="center">
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/Tauri-v2-24C8D5?logo=tauri&logoColor=white" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.75+-black?logo=rust&logoColor=white" /></a>
  <a href="https://vuejs.org"><img src="https://img.shields.io/badge/Vue-3.x-4FC08D?logo=vue.js&logoColor=white" /></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Android-blue" />
  <img src="https://img.shields.io/badge/License-MIT-yellow" />
</p>
---

## 🚩 普通用户请先看这里

如果你是 **同人社团摊主 / 使用者**，正在寻找：

- 安装包下载  
- 使用教程  
- FAQ / 现场排障  

👉 请访问 **官方文档与下载站点**：  
**https://boothkernel.secret-sealing.club**

> 本 GitHub 仓库主要面向 **开发者 / 维护者 / 贡献者**，包含源代码与技术文档。

---

## ✨ 项目概述

**摊盒（Booth-Kernel）** 是一款专为 **漫展 / 同人展 / 校园集市** 场景设计的本地化出摊系统。

它解决的不是“功能不够多”，而是几个在现场极其致命的问题：

- 场馆网络拥堵 / 无信号  
- 纸笔记账易错、难复盘  
- 小程序/云服务依赖网络与平台  

### 设计核心

> **离线优先 · 局域网架构 · 本地数据**

系统采用 **主机 / 客户端（浏览器）** 的局域网架构：

- **Host（主机端）**
  - Tauri 桌面/移动应用
  - 内置 Axum HTTP 服务
  - SQLite 本地数据库
  - 负责：业务逻辑、库存、订单、导出

- **Client（客户端）**
  - 局域网内任意设备（手机 / 平板）
  - 直接通过浏览器访问
  - 无需安装 App
  - 用作：顾客点单屏 / 摊主接单终端

---

## 🔑 核心特性

- **离线优先**  
  所有功能均可在 **无互联网** 情况下运行，仅依赖局域网。

- **数据完全本地化**  
  所有数据存储于本机 SQLite 数据库（`data.db`），不上传、不收集、不分析。

- **高可靠性**  
  SQLite 实时落盘 + Rust 后端，即使异常退出也能恢复数据。

- **低资源占用**  
  Rust + Tauri 架构，内存/CPU 占用远低于 Electron 方案。

- **跨平台**  
  当前稳定支持 Windows / Android（Linux & macOS 适配中）。

---

## 🧠 技术架构

| 层级 | 技术 | 说明 |
|----|----|----|
| 应用框架 | **Tauri v2** | 跨平台容器，Rust + WebView |
| 后端 | **Rust** | 核心逻辑、系统能力 |
| HTTP | **Axum** | 嵌入式 REST API |
| 数据库 | **SQLite + SQLx** | 本地存储，异步 ORM |
| 前端 | **Vue 3 + Vite** | SPA 界面 |
| UI | **Naive UI** | 统一、轻量、可主题化 |
| 状态 | **Pinia** | 前端状态管理 |

---

## 💻 开发指南（Development）

### 环境要求

- **Node.js** ≥ 18
- **Rust** ≥ 1.75（推荐 stable via rustup）
- **Package Manager**：`pnpm`
- **平台工具链**
  - Windows：Visual Studio C++ Build Tools
  - Android：Android Studio + SDK / NDK

---

### 快速启动（Desktop）

```bash
git clone https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp.git
cd ABL-BoothApp
pnpm install
pnpm tauri dev
````

> 该命令会同时启动：
>
> * Vite 前端 dev server
> * Tauri + Rust 后端

---

### Android 开发

```bash
pnpm tauri android init
pnpm tauri android dev
```

---

### 构建（Build）

#### Windows

```bash
pnpm tauri build
```

#### Android

```bash
pnpm tauri android build
```

> ⚠️ Android 构建说明：
>
> * 默认仅支持 **arm64-v8a / armeabi-v7a**
> * 模拟器（x86）可能无法运行
> * 签名配置位于 `src-tauri/gen/android/keystore.properties`

#### 构建更小的 APK（推荐）

```bash
# 现代设备
npm run tauri -- android build --apk true -t aarch64

# 较老设备
npm run tauri -- android build --apk true -t armv7
```

---

## 📂 项目结构

```text
.
├── frontend/                 # Vue 前端
│   ├── views/           # Admin / Vendor / Customer
│   ├── components/
│   ├── stores/
│   └── services/
├── src-tauri/           # Rust 后端
│   ├── src/
│   │   ├── main.rs
│   │   ├── db/
│   │   ├── http/
│   │   └── utils/
│   ├── tauri.conf.json
│   └── capabilities/
└── docs/                # 文档站（VitePress）
```

---

## 🤝 贡献指南（Contributing）

欢迎任何形式的贡献，包括但不限于：

* Bug 修复
* 文档改进
* UI / UX 优化
* 新功能提案

流程：

1. Fork 本仓库
2. 新建分支：`feat/xxx`
3. 提交符合语义化规范的 commit
4. 发起 Pull Request

---

## 🔐 安全与信任声明

* 本项目 **不接入任何支付接口**
* 不采集、不上传、不分析任何交易或营业数据
* 所有数据仅存在于用户本地设备
* 即使项目停止维护，现有版本依然可长期使用

---

## 📄 License

MIT License
请保留原作者与项目来源信息。

我们 **不推荐** 将本项目包装为闭源或付费商业软件销售，
但你拥有 MIT 协议赋予的自由。

---

## 👤 作者 / 核心维护

- **Renko_1055**  
  项目发起、核心架构与主要开发  
  GitHub: https://github.com/Renko6626

  ## 🙏 致谢

感谢以下东方Project同人社团在测试、设计与建议上的支持（排名不分先后）：

- 境界景观学会(同人社团) — 压力测试 / 现场使用反馈 
- 东方幻想指南 — 提供鼓励和支持，并深度参与测试与反馈
- 第零研究院、墨斯卡林之翼 — 在多次展会中使用项目的早期版本并提供宝贵反馈

感谢维生素X绘制教程页面的插画素材。
---

Built with ❤️ for doujin circles.

