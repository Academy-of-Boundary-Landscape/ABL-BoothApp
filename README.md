<p align="center">
  <img src="docs/images/app-icon.png" width="120" alt="Booth-Kernel Logo" />
</p>

<h1 align="center">摊盒 Booth-Kernel</h1>

<p align="center">
  <b>LAN-first · Offline · Local-first 出摊系统</b><br/>
  基于 Tauri v2 + Rust 的现代化同人展会收银与库存管理工具
</p>

<p align="center">
  <a href="https://boothkernel.secret-sealing.club">官网</a> ·
  <a href="https://boothkernel.secret-sealing.club/guide/getting-started">使用文档</a> ·
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

## 🎉 v1.1.0 重大更新

> **摊盒 v1.1.0 现已发布！** 这是一次重量级更新，跨越数月开发，从"能用"进化到"好用"。

亮点：

- 📸 **AI 拍照识别**：对准商品拍一张照，系统自动识别加入购物车。不用贴条形码、不用背 SKU——**专为"帮朋友看摊/寄售"这种"自己都认不全货"的场景设计**。底层基于 ONNX Runtime 的图像嵌入搜索，支持 GPU / NPU 硬件加速
- 🏷️ **多维标签**：除了商品分类，现在可以按角色、IP、系列等多个维度快速筛选
- 🎨 **界面全面翻新**：统一的设计 Token 系统，深色模式适配，触控尺寸优化
- 📱 **Android 版本打磨**：NNAPI 硬件加速，arm64-v8a APK 构建流程成熟
- 🛡️ **并发安全重铸**：SQLite 切换到 WAL 模式，库存扣减改为原子操作，多人同时下单不再丢数据
- 🧭 **上手更容易**：快速上手进度条、空白页引导、页面帮助气泡，新摊主零学习成本

👉 **完整变更日志**：[CHANGELOG-v1.1.md](./CHANGELOG-v1.1.md)

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

它解决的是几个在现场极其致命的问题：

- 场馆网络拥堵 / 无信号  
- 纸笔记账易错、难复盘
- 管理和辨别大量商品的心智负担  
- 小程序/云服务依赖网络与平台  

在最新的v1.1.x版本, 我们引入了**AI 图像识别**功能。通过内嵌的神经网络, 可以让顾客/摊主调用摄像头拍照，识别最接近的商品并进行后续操作。
这极大降低了摊主和顾客的心智负担 —— 不再需要刻意记忆sku、制品名、制品图片了。

### 为什么 AI 拍照识别，而不是条形码？

传统零售系统依赖条形码，因为商品稳定、由上游生产、长期复用。**但同人摊恰好相反**：

- 二次元同人制品种类众多但单品量少，条码基本只能人力手动打印和粘贴，非常痛苦。
- 对于较小亚克力＆明信片类制品，条形码难以贴附且不美观。
- 如果你给他人看摊，对方连SKU都不一定有，条码基本不能指望，而视觉方案只需要临时手机拍照就能工作。

只要预先给商品拍几张照（毕竟宣传总需要拍失误），开摊后拍照即结算。再也不需要记忆 “这个东西对应商品的名称和SKU是什么“了。

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

- **离线优先** — 所有功能无需互联网，仅依赖局域网
- **数据本地化** — 所有数据存储在本机 SQLite，不会上传给任何其他人
- **高可靠性** — SQLite WAL 模式 + 原子事务，多人同时下单不丢数据，异常退出也能恢复
- **AI 视觉识别** — ONNX Runtime 驱动的图像嵌入搜索，拍照即可找到商品
- **低资源占用** — Rust + Tauri 架构，相对 Electron 方案更轻量
- **跨平台** — 稳定支持 Windows 和 Android（arm64-v8a）

---

## 💻 给开发者

### 技术栈

| 层级 | 技术 |
|------|------|
| 应用框架 | Tauri v2（Rust + WebView） |
| 后端 | Rust + Axum（嵌入式 HTTP 服务） |
| 数据库 | SQLite + SQLx（WAL 并发模式） |
| AI 推理 | ONNX Runtime（动态加载，CPU / DirectML / NNAPI） |
| 前端 | Vue 3 + Vite + Naive UI + Pinia |

### 快速启动

```bash
git clone https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp.git
cd ABL-BoothApp
npm install
npm run tauri dev
```

更详细的环境配置、Android 交叉编译、签名打包等内容见 [**docs/BUILD.md**](./docs/BUILD.md)。

最新变更见 [**CHANGELOG-v1.1.md**](./CHANGELOG-v1.1.md)。

---

## 🤝 贡献

欢迎通过 Issue 反馈 Bug 或通过 Pull Request 提交改进。开发流程细节见 [docs/BUILD.md](./docs/BUILD.md)。

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

- **Renko_1055** — 项目发起、核心架构与主要开发  
  GitHub: https://github.com/Renko6626

---

## 🙏 致谢

感谢以下东方Project同人社团在测试、设计与建议上的支持（排名不分先后）：

- 境界景观学会(同人社团) — 压力测试 / 现场使用反馈 
- 东方幻想指南 — 提供鼓励和支持，并深度参与测试与反馈
- 第零研究院、墨斯卡林之翼 — 在多次展会中使用项目的早期版本并提供宝贵反馈

感谢维生素X绘制教程页面的插画素材。

---

Built with ❤️ for doujin circles.

