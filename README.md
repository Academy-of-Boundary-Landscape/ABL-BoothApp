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
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.94+-black?logo=rust&logoColor=white" /></a>
  <a href="https://vuejs.org"><img src="https://img.shields.io/badge/Vue-3.x-4FC08D?logo=vue.js&logoColor=white" /></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Android-blue" />
  <img src="https://img.shields.io/badge/License-MIT-yellow" />
</p>

---

## 🎉 v1.2 更新：复式账本

> v1.1 让摊盒「能用」，v1.2 让它**记得清账**。这一版重写了账本：每一件货、每一分钱都有来处和去处。

- 📒 **复式记账**：下单、收款、退货、赠送、报废、盘点，每一笔都是借贷必平的分录，货和钱都能追到来去
- 🎁 **套装与最优折扣**：「任选 3 张 20」「新刊 + 立牌 一起 50」，顾客购物车自动求出最省组合并写明省了多少；摊主收款时可拆套装、改实收
- 🧾 **收摊与结算**：清点订单 → 盘点 → 带回 → 结算 四步向导，一张结算单算清每个社团（货主）该分多少、我应转给谁多少，寄售分账不再手算
- ▦ **条码扫描**：摄像头连续扫码加购、支持扫码枪；商业条码（JAN/ISBN）和自己贴的编号标签都认，全程离线
- 🧭 **重新设计的界面**：展会工作台按「展前 / 现场 / 收摊」组织，「从上一场导入」一键沿用商品和套装；摊主端改为订单 / 库存 / 收摊三个 tab，手机上单手可用

> ⚠️ **从 v1.1 升级**：旧版的展会和订单不会迁移到新账本（商品库会自动带过来）。旧数据完整保留，可随时导出为 Excel。
> 请先阅读 [从 v1.1 升级到 v1.2](https://boothkernel.secret-sealing.club/guide/upgrade-v1.2)。

👉 **完整更新说明**：[docs/releases/v1.2.0.md](./docs/releases/v1.2.0.md) · 历史版本：[v1.1.1](./docs/releases/v1.1.1.md) / [v1.1.0](./docs/releases/v1.1.0.md)

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
- 纸笔记账易错、难复盘，寄售分账全靠手算
- 管理和辨别大量商品的心智负担
- 小程序 / 云服务依赖网络与平台

### 一场展会的完整闭环

```
展前                        现场                                   收摊
商品库 → 建展会 → 上架/套装   顾客自助点单 → 摊主确认收款 → 退货/赠送   清点 → 盘点 → 带回 → 结算单
```

- **展前**：商品库是可复用的模板；每场展会从商品库选、或从上一场导入，配好套装，点「开始展会」
- **现场**：平板摆在摊位前当点单页（浏览 / 扫码 / 拍照识别加购），摊主手机接单提醒、确认收款
- **收摊**：摊主端走四步向导，生成按社团分块的结算单，导出 Excel 分账

### 找商品：拍照识别 + 条码扫描

同人制品种类多、单品量少，大多没有条码，逐件贴码又费事又难看；帮朋友看摊的人更是连 SKU 都不认识。所以摊盒提供两条路：

- **AI 拍照识别**：预先给商品拍几张照，现场拍一下就能认出来。基于 ONNX Runtime 的图像嵌入搜索，模型下载后完全离线
- **条码扫描**（v1.2）：商业本、官方周边自带的 JAN / ISBN 直接扫；自己的制品把编号打印成条码贴上也能扫。支持扫码枪

### 设计核心

> **离线优先 · 局域网架构 · 本地数据**

系统采用 **主机 / 客户端（浏览器）** 的局域网架构：

- **Host（主机端）**
  - Tauri 桌面 / 移动应用（Windows / Android）
  - 内置 Axum HTTP / HTTPS 服务
  - SQLite 本地数据库（复式账本）
  - 负责：业务逻辑、账本、库存、订单、结算、导出

- **Client（客户端）**
  - 局域网内任意设备（手机 / 平板）
  - 直接通过浏览器访问，无需安装 App
  - 用作：顾客点单屏 / 摊主接单终端 / 远程管理后台

---

## 🔑 核心特性

- **离线优先**：所有功能无需互联网，仅依赖局域网（手机热点即可）
- **数据本地化**：所有数据存储在本机 SQLite，不会上传给任何人
- **账目可追溯**：复式记账，借贷必平写成不变量测试；结算单与账本对不上时页面直接警示
- **寄售友好**：商品按社团（货主）归属，结算单自动分块，手工让价不影响代卖方分成
- **高可靠性**：SQLite WAL + 原子事务；每次升级前自动快照，迁移失败自动回滚
- **AI 视觉识别 + 条码扫描**：拍照或扫码都能找到商品
- **低资源占用**：Rust + Tauri 架构，相对 Electron 方案更轻量
- **跨平台**：稳定支持 Windows（x64）和 Android（arm64-v8a）

---

## 💻 给开发者

### 技术栈

| 层级 | 技术 |
|------|------|
| 应用框架 | Tauri v2（Rust + WebView） |
| 后端 | Rust + Axum（嵌入式 HTTP / HTTPS 服务），utoipa 生成 OpenAPI |
| 数据库 | SQLite + SQLx 0.9（WAL，复式账本，迁移前自动快照） |
| AI 推理 | ONNX Runtime 1.23（动态加载，CPU / DirectML / NNAPI） |
| 前端 | Vue 3 + TypeScript（strict）+ Vite + Naive UI + Pinia，API 类型由 `openapi.json` 生成 |
| 测试 | cargo test（handler 级集成测试，内存 SQLite）、Vitest、CI 五个 job |

### 快速启动

```bash
git clone https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp.git
cd ABL-BoothApp
./scripts/setup-dev.sh      # 安装两处 npm 依赖、下载 ONNX Runtime 原生库、清点签名材料
npm run tauri dev
```

> 前端在 `frontend/`（有自己的 `package.json`），仓库根的 `package.json` 只放 Tauri CLI 和 VitePress 文档站，**两处都要装依赖**。

### 常用命令

```bash
(cd src-tauri && cargo test)                   # 后端测试
npm --prefix frontend run test:unit            # 前端单测
npm --prefix frontend run lint                 # eslint + stylelint（设计 token 门禁）+ UI 原语边界检查
npm --prefix frontend run typecheck            # vue-tsc strict

# 改了后端接口：更新契约快照 → 重新生成前端类型
(cd src-tauri && UPDATE_OPENAPI=1 cargo test --all-features openapi_snapshot)
npm --prefix frontend run gen:api

npm run docs:dev                               # 本地预览文档站
```

更详细的环境配置、Windows / Android 交叉编译、签名打包、发布流程见 [**docs/BUILD.md**](./docs/BUILD.md)。
**已发布的数据库迁移文件永久冻结**，改动前务必先读 BUILD.md 的「发布前必查」。

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

