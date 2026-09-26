# 点单页条码扫描 设计

**日期**: 2026-09-26
**作者**: Renko_6626（与 Claude 协作）
**状态**: Draft
**前置阅读**: `2026-09-22-v1.2-roadmap.md` 附录八（拍照识别修复与剩余待办）

---

## 1. 目标

给点单页加一个「扫码」入口：摄像头对准商品条码，商品直接进购物车。

- **主要用户是摊主**。摊主端没有自己的点单界面，「去点单」就是顾客点单页（`CustomerView`），
  代点单、收银都在这里。一台平板屏幕朝顾客时，后置摄像头正好朝摊主——摊主把商品举到平板背后扫，
  顾客在屏幕上看着购物车变化。
- **摄像头优先**。绝大多数同行没有扫码枪；扫码枪是第二步。
- **入口不设闸门**。顾客也能看到、也能用，不按登录身份区分。
- 成功标准：摊主在安卓平板上用后置摄像头连续扫一摞商品，每件「嘀」一声进购物车，不重复加、
  扫不到的不打断下一件；全程不联网。

## 2. 条码从哪来

商品有两种：

- **自带商业条码**（JAN/EAN-13、ISBN）：官方周边、商业本。
- **没有商业条码**：同人本、吧唧等，大多靠摊主自己贴标签，标签内容就是 `product_code`。

所以扫描时的约定是：**有商业条码按商业条码找，没有就按 `product_code` 找**（SKU 即条码）。

## 3. 数据

### 3.1 字段

`master_products` 新增 `barcode TEXT`（可空）+ **非唯一**索引。新增一个迁移文件，不动已发布的迁移
（见 `docs/BUILD.md`「发布前必查」）。

非唯一的理由：同一个 JAN 对应多件商品不常见但存在（同一印刷品的不同版本），命中多件时让人挑（§4.3），
不在录入时报错。

### 3.2 读写路径

| 路径 | 改动 |
|---|---|
| 全局商品库：列表 / 新建 / 编辑（`api/master_product.rs`、`CreateMasterProductForm.vue`、`EditMasterProductModal.vue`） | 读写 `barcode`，表单加「商业条码（可选）」输入框 |
| 展会商品列表（`api/product.rs` 的 `EP_ROW_BY_ID` / `EP_ROWS_BY_EVENT`） | 多 JOIN 出 `mp.barcode`，`EventProductResponse` 加字段 |
| `.boothpack` 导出 / 导入（`api/sync.rs`，`MasterProduct` 的 serde） | 带上 `barcode`；导入时**包里没有就不覆盖**本机已有值（`COALESCE(excluded.barcode, master_products.barcode)`） |

兼容：旧包没有这个字段 → `#[serde(default)]` 为 `None` → 不覆盖；旧版 App 导入新包 → serde 默认
忽略未知字段（已确认仓库里没有 `deny_unknown_fields`）。

### 3.3 保存时规范化

去掉首尾空格；只由数字、空格、连字符组成时（手敲的 `978-4-…`）去掉空格和连字符；空串存 `NULL`。
**不校验校验位**，免得误拦。

## 4. 匹配规则

匹配在**前端**做：点单页本来就把本场全部商品加载在 `customerStore.products` 里，本地查，断网也能扫，
也最快。只在本场商品里找。

### 4.1 顺序

1. 扫描文本去首尾空格。
2. **日本书籍的分类价格码直接忽略**：13 位、以 `191` 或 `192` 开头。日本的书上下两个条码，
   下面那个是它；不忽略的话每扫一本书都闪一次红。返回「忽略」，界面无任何反应。
3. **按商业条码找**：命中一件或多件就用这些，不再看 `product_code`。
   比较时 12 位 UPC-A 与前补 `0` 的 13 位 EAN-13 视为同一个码（不同识别引擎对同一张码的输出不一致）。
4. **按 `product_code` 找**：不区分大小写；本场内唯一，最多一件。
5. 都没有：「未找到」。

### 4.2 命中之后

| 结果 | 表现 |
|---|---|
| 一件、有货 | 加入购物车，「嘀」一声，框闪绿，最近记录加一行「+1 本子A ¥30」 |
| 一件、已售罄 | 框闪红，提示行「本子A 已售罄」 |
| 多件 | 暂停扫描，弹小列表选一件，选完或取消后继续扫 |
| 未找到 | 框闪红，提示行「未找到条码 4901234567894」 |
| 忽略 | 无反应 |

「已售罄」的判定、加购物车都走现有的 `store.addToCart` 与 `onsite_qty`（和拍照识别的 `onVisionSelect` 一致）。

### 4.3 不做

- 标签打印（给没有商业条码的商品按 `product_code` 印码）。
- 扫到本场没有的码时跳转「新建商品」。

等扫码用起来再看要不要。

## 5. 点单页 UI

### 5.1 入口

- 工具栏中间原来是「商品列表 | 拍照识别」两段等宽切换。改为：「商品列表」是主视图；
  旁边两个**小一级**的次级按钮「📷 拍照识别」「▦ 扫码」。进入任一面板后有明显的「返回商品列表」。
- 吸引屏：大按钮只留「开始点单」，下面一行小字链接「拍照识别」「扫码」。
- 拍照识别不可用时仍置灰（沿用 `refreshVisionAvailability`）。扫码不依赖后端模型；设备没有摄像头
  （`enumerateDevices` 里没有 videoinput，或不是安全上下文）时置灰并提示原因。

### 5.2 扫码面板

- 布局同拍照识别面板：取景占商品网格的位置，购物车保持可见（宽屏侧栏 / 平板与手机底部条）。
- **连续扫描**，无快门。默认**后置**摄像头（`facingMode: 'environment'`），可翻转；
  track 支持 `torch` 时显示补光灯开关。
- 取景框是**横向长条**（适合 1D，QR 放进去也能认），只解码框内区域，约每 200ms 一次。
- **防重复**：同一个码要先离开画面（连续若干帧没检测到），或距上次超过 1.5 秒，才再算一次。
- **失败不弹窗**，只闪红 + 提示行，弹窗会打断连续扫描。唯一的弹出是「多件命中」的选择列表。
- 提示音用 Web Audio 生成的短促提示音，不复用新单提示音 `notify.mp3`。
- 离开扫码面板、回到吸引屏、页面进入后台（`visibilitychange`）时关闭摄像头。

## 6. 技术实现

### 6.1 识别引擎

- 依赖 `barcode-detector`（3.2.x，底层 `zxing-wasm`，ZXing C++ 编译的 WASM）。
  用 `barcode-detector/ponyfill`，不用 polyfill，不改全局对象。
- **wasm 必须自带**：它默认从 jsDelivr CDN 拉 `zxing_reader.wasm`（约 1.1MB），场馆常常断网。
  Vite 以 `?url` 引入该文件，用 `prepareZXingModule({ overrides: { locateFile } })` 指向它。
  于是 App 内（Tauri 资源协议）和 LAN 浏览器（本机 HTTP 服务）都从本机加载。
- 浏览器自带 `BarcodeDetector` 且 `getSupportedFormats()` 覆盖所需格式时优先用原生（Chrome Android、
  macOS），否则用 WASM（Windows WebView2、iOS）。两者同一 API。
- 扫码面板首次打开时才动态 `import()`，点单页首屏不受影响。
- 格式：`ean_13`、`ean_8`、`upc_a`、`upc_e`、`code_128`、`code_39`、`qr_code`。

### 6.2 模块

| 单元 | 职责 | 依赖 |
|---|---|---|
| `utils/barcodeMatch.ts` | 纯函数：`(扫描文本, 本场商品) → 命中列表 / 忽略 / 未找到`。§4.1 全部规则 | 无 |
| `composables/useCamera.ts` | 取流、翻转、补光灯、释放。**任何时候卸载都不漏 track**：取流进行中卸载、连点翻转，拿到的流都要 stop | `getUserMedia` |
| `composables/useBarcodeScanner.ts` | 懒加载引擎、按节奏解码取景框区域、防重复；只对外发「扫到码 X」 | `useCamera`、引擎 |
| `components/customer/BarcodeScanPanel.vue` | 取景、闪框、提示音、最近记录、多件选择 | 上面三个 |

`VisionSearch.vue` 的取流代码改用 `useCamera`，顺带修掉附录八里「摄像头 track 泄漏」这一条。

### 6.3 待实测的风险

- `.wasm` 的 Content-Type：本机 HTTP 服务（`web/` 的 `mime_guess`）与 Tauri 资源协议是否给
  `application/wasm`。不是的话 `instantiateStreaming` 失败退回较慢路径并报警告，要修。
- Windows 笔记本前置摄像头多为定焦，1D 码可能很难对上焦。界面提示「条码离镜头远一点」，不另做处理。

## 7. 分步交付

**第一步**（本 spec 的主体）：`barcode` 字段与读写路径、匹配规则、扫码面板、`useCamera` 抽取、
入口布局调整。

**第二步**：

- **扫码枪**：点单页监听键盘，「极短间隔的连续按键 + 回车」视为一次扫码，走同一个 `barcodeMatch`。
  输入框聚焦时不拦截。
- **表单「扫码填入」**：商品编辑表单的条码输入框旁加按钮，复用扫码面板的单次模式，免得手敲 13 位。

## 8. 测试

- `barcodeMatch` 单测：每条规则一例以上——规范化、UPC-A/EAN-13 等价、191/192 忽略、
  商业条码优先于 `product_code`、`product_code` 大小写、多件命中、未找到。
- `useCamera`：mock `getUserMedia`，覆盖「取流完成前卸载」「连续翻转」后所有 track 都被 stop。
- `BarcodeScanPanel` 组件测试（mock 识别器）：防重复、失败闪红不弹窗、多件命中暂停扫描、
  离开面板停摄像头。
- 后端：新建 / 编辑的条码读写与规范化、展会商品列表带出条码、`.boothpack` 往返、旧包不清空本机条码；
  更新 OpenAPI 快照并重新生成前端类型。
- 用 Node 加载**打包进来的那份** wasm 解码几张示例条码图（EAN-13、UPC-A、Code128、QR），
  证明自带路径可用、不依赖网络。
- 真机：安卓平板后置、Windows 笔记本摄像头、iPhone Safari 走 LAN，各扫 EAN-13 与 QR；
  断网再扫一遍。
