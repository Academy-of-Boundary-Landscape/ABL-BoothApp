# 点单页条码扫描 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 点单页新增「扫码」入口：摄像头（第二步加扫码枪）扫商品条码，按「商业条码优先、否则 product_code」匹配本场商品并加入购物车。

**Architecture:** 后端只给 `master_products` 加可空 `barcode` 列并打通读写路径；匹配在前端对已加载的本场商品做。扫描链路分三层：`useCamera`（取流/释放）→ `useBarcodeScanner`（懒加载 `barcode-detector` 引擎、节奏解码、防重复）→ `BarcodeScanPanel.vue`（交互）。wasm 随前端打包，全程不联网。

**Tech Stack:** Rust / axum / sqlx（SQLite）；Vue 3 + TS + naive-ui；`barcode-detector@3.2.x`（`zxing-wasm` 3.1.x）；vitest + jsdom。

**Spec:** `docs/superpowers/specs/2026-09-26-barcode-scan-design.md`（执行者必须先读它；本 plan 只写契约与必测项）

## Global Constraints

- 已发布的迁移文件一律不改，只新增（`docs/BUILD.md`「发布前必查」）。新迁移文件名：`src-tauri/migrations/202609260001_add_master_product_barcode.sql`。
- 后端命令必须带前缀：`cd src-tauri && tauri-env linux cargo …`（裸 `cargo` 缺 webkit2gtk 必失败）。
- 改了后端接口：`(cd src-tauri && UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot)`，再 `npm --prefix frontend run gen:api && npm --prefix frontend run typecheck`。
- 前端门禁：`npm --prefix frontend run lint`（eslint + stylelint token 门禁 + 原语边界）与 `cd frontend && npx vitest run` 全绿。样式只用 token（`var(--space-*)`、`var(--font-*)` 等），不写裸颜色/px 间距；门禁会拦。
- **绝对不要跑**永不退出的命令：`tauri dev`、`npm run dev`、`vite`（非 build）、`vncserver`。凡「在 VNC/浏览器里看一眼」「手动确认」的步骤一律跳过，由控制者做。验证前端只用会退出的 `npm --prefix frontend run build`。
- wasm 不得从网络加载：`barcode-detector` 默认 `locateFile` 指向 jsDelivr，必须覆盖为本地打包资源。
- 条码字段名全链路统一叫 `barcode`（DB 列、Rust 字段、JSON、TS）。
- 提交信息沿用仓库风格：`✨ feat: …` / `🐛 fix: …` / `♻️ refactor: …`，结尾加
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`。

## Review Focus

1. **商品停在镜头前被连续多帧识别**：只能加一件；移开再拿回来（或超过 1.5 秒）才算第二件。→ Task 4 的组件测试。
2. **取流进行中切走页面 / 连点「翻转」**：摄像头灯必须熄灭，不能残留 track。→ Task 3 的 `useCamera` 测试。
3. **断网**：wasm 必须从本地资源加载，不能请求 CDN。→ Task 4 断言 `prepareZXingModule` 被调用且 `locateFile('zxing_reader.wasm')` 返回的不是 http(s) 外链；Node 测试直接用 `node_modules` 里那份 wasm 解码。
4. **旧版 `.boothpack`（没有 barcode 字段）导入**：不能清掉本机已填的条码。→ Task 1 的后端测试。
5. **扫码枪在输入框里打字时**：不能被当成扫码拦截（例如购物车备注、搜索框）。→ Task 5 的测试。

---

## 批次与并行

- **批次 A（并行，三个独立 worktree）**：Task 1（后端 + 表单字段）、Task 2（匹配函数）、Task 3（`useCamera` + VisionSearch 迁移）。文件互不重叠。
- **批次 B（串行，在合并后的主分支上）**：Task 4 → Task 5。

---

### Task 1: `barcode` 字段与读写路径

**Files:**
- Create: `src-tauri/migrations/202609260001_add_master_product_barcode.sql`
- Modify: `src-tauri/src/db/models.rs`（`MasterProduct`）
- Modify: `src-tauri/src/api/master_product.rs`（create / update 的 multipart 解析、两个 OpenAPI 表单结构、INSERT/UPDATE SQL）
- Modify: `src-tauri/src/api/product.rs`（`EP_ROW_BY_ID`、`EP_ROWS_BY_EVENT`、`EventProductRow`、`EventProductResponse`）
- Modify: `src-tauri/src/api/sync.rs`（导入的 upsert SQL）
- Create or modify: 规范化函数放在 `src-tauri/src/utils/` 下合适的位置（如新建 `utils/barcode.rs` 并在 `utils/mod.rs` 注册）
- Modify: `frontend/src/components/product/CreateMasterProductForm.vue`、`frontend/src/components/product/EditMasterProductModal.vue`
- Regenerate: `src-tauri/openapi.json`、`frontend/src/api/schema.d.ts`

**Interfaces:**
- Produces（DB）：`master_products.barcode TEXT NULL` + `CREATE INDEX idx_master_products_barcode ON master_products(barcode)`（非唯一）。
- Produces（Rust）：`MasterProduct.barcode: Option<String>`，带 `#[serde(default)]`；`pub fn normalize_barcode(raw: &str) -> Option<String>`。
- Produces（API）：`MasterProduct` JSON 多 `barcode: string | null`；`ProductEventProduct`（`EventProductResponse`）多 `barcode: string | null`（取 `mp.barcode`）；create/update 的 multipart 表单多可选字段 `barcode`。
- Produces（TS）：`Schemas['ProductEventProduct']['barcode']: string | null`，Task 2/4 依赖。

**规则：**
- `normalize_barcode`：trim；空串 → `None`；若只由 ASCII 数字、空格、`-` 组成 → 去掉空格与 `-`；否则原样（已 trim）。不校验校验位。
- create：表单有 `barcode` 就规范化后写入，没有写 NULL。
- update：表单**出现** `barcode` 字段就用规范化结果覆盖（空串 → NULL）；**没出现**就保持原值。
- `.boothpack` 导入 upsert：INSERT 带 `barcode`，`ON CONFLICT` 里写 `barcode = COALESCE(excluded.barcode, master_products.barcode)`。导出无需改（`MasterProduct` 序列化自带）。
- 前端两个表单：在 product_code 下方加「商业条码（可选）」输入框（`n-input`，clearable，placeholder「JAN / ISBN 等，没有就留空」），提交时总是 append `barcode`（空串表示清空）。编辑表单回填现有值。

**必测（后端，放在对应文件已有的 `#[cfg(test)]` 模块里，沿用 `test_support` 的夹具）：**
- `normalize_barcode`：`" 978-4-06-123456-7 "` → `"9784061234567"`；`"ab-12 C"` → `"ab-12 C"`；`"  "` → `None`。
- create 带 `barcode` → 响应与 DB 里是规范化值；不带 → `null`。
- update 带空串 `barcode` → 变 NULL；update 不带 `barcode` → 原值不变。
- 展会商品列表 `GET /api/events/{id}/products` 的元素带出 `barcode`。
- `.boothpack` 导入：本机商品已有 barcode，导入一个该商品不含 barcode 字段的包 → barcode 保持；包里带 barcode → 被更新。（参考 `api/sync.rs` 的 `make_pack` 与 `import_resolves_owner_by_society_name…` 测试写法。）
- 已有 shape 测试（`shape_of` 快照）因新字段变红的，按新形状更新。

**验收：**
```bash
cd src-tauri && tauri-env linux cargo test --all-features
cd src-tauri && UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot
cd src-tauri && tauri-env linux cargo clippy --all-features --all-targets
cd src-tauri && tauri-env linux cargo clippy --no-default-features --all-targets
npm --prefix frontend run gen:api && npm --prefix frontend run typecheck && npm --prefix frontend run lint
cd frontend && npx vitest run
```
提交：`✨ feat: 商品库新增商业条码字段 barcode`

---

### Task 2: 匹配函数 `barcodeMatch`

**Files:**
- Create: `frontend/src/utils/barcodeMatch.ts`
- Test: `frontend/src/utils/barcodeMatch.spec.ts`

**Interfaces:**
- Consumes：商品只需要 `{ product_code: string; barcode?: string | null }` 这两个字段（用泛型，别依赖 Task 1 生成的类型，以便并行）。
- Produces：

```ts
export type BarcodeMatch<T> =
  | { kind: 'hit'; products: T[] }   // products.length >= 1
  | { kind: 'ignored' }              // 日本书籍分类价格码
  | { kind: 'not_found'; code: string }

export function matchBarcode<T extends { product_code: string; barcode?: string | null }>(
  raw: string,
  products: readonly T[],
): BarcodeMatch<T>
```

**规则（spec §4.1，按顺序）：**
1. `code = raw.trim()`；空串 → `not_found`（code 为空串）。
2. `/^19[12]\d{10}$/` → `ignored`。
3. 商业条码：用规范化键比较——若是纯数字且长度 12，键为 `'0' + code`；纯数字 13 位保持；其它原样。商品侧 `barcode` 同样处理（null/空跳过）。命中 ≥1 件 → `hit`（保持商品原有顺序）。
4. 否则 `product_code` 比较（两边 trim、`toLowerCase()`），命中 → `hit`（1 件）。
5. 否则 `not_found`。

**必测：**每条规则至少一例；另加：商业条码命中时不再看 product_code（某商品 product_code 恰好等于扫描值、另一商品 barcode 也等于它 → 只返回后者）；两件共用同一 barcode → `hit` 两件；UPC-A 12 位扫描值命中存的 13 位 `0…`，反之亦然；`" p001 "` 命中 product_code `P001`；`9784061234567` 不被当作价格码忽略；`1920123456789` 被忽略。

**验收：** `cd frontend && npx vitest run src/utils/barcodeMatch.spec.ts`，`npm --prefix frontend run lint`，`npm --prefix frontend run typecheck`。
提交：`✨ feat: 条码匹配规则 matchBarcode`

---

### Task 3: `useCamera` 抽取，VisionSearch 改用它（修 track 泄漏）

**Files:**
- Create: `frontend/src/composables/useCamera.ts`
- Test: `frontend/src/composables/useCamera.spec.ts`
- Modify: `frontend/src/components/shared/VisionSearch.vue`（`startCamera` / `stopCamera` / `switchCamera` / `currentStream` / `currentFacing` 改为调用 `useCamera`；其余逻辑与模板行为不变）

**Interfaces:**
- Produces：

```ts
export type Facing = 'user' | 'environment'
export function useCamera(opts: {
  facing: Facing
  constraints?: MediaTrackConstraints   // 合并进 video 约束，默认 { width: { ideal: 1280 }, height: { ideal: 960 } }
}): {
  stream: Readonly<Ref<MediaStream | null>>
  isActive: Readonly<Ref<boolean>>
  facing: Ref<Facing>
  error: Ref<string>                     // 中文，可直接展示；成功时为 ''
  torchSupported: Readonly<Ref<boolean>>
  torchOn: Readonly<Ref<boolean>>
  start(): Promise<boolean>              // 成功返回 true
  stop(): void
  flip(): Promise<boolean>
  setTorch(on: boolean): Promise<void>
}
```

**行为要求：**
- 非安全上下文或没有 `navigator.mediaDevices.getUserMedia`：`start()` 返回 false，`error` 为 VisionSearch 现有那段「当前页面不是安全连接…」文案（原样搬过来）。
- **不泄漏**：用一个递增的请求序号；`getUserMedia` resolve 时如果序号已过期（期间调用过 `stop()` / 又一次 `start()` / 组件已卸载），立刻 stop 这个新流的所有 track 并丢弃。`onScopeDispose` 里调用 `stop()`。
- `flip()`：切换 `facing` 后重新 `start()`；连点只保留最后一次的流。
- `torchSupported`：从视频 track 的 `getCapabilities?.().torch` 取；`setTorch` 用 `applyConstraints({ advanced: [{ torch: on }] })`，失败静默置回 false。
- 权限被拒：`error` = `'无法访问摄像头: ' + (e.message || e.name)`（沿用现有文案）。
- VisionSearch：`<video>` 的 `srcObject` 在 `stream` 变化时（`watch` + `nextTick`）绑定；镜像判断改读 `facing`；外部行为、模板 class、现有测试不变。

**必测（mock `navigator.mediaDevices.getUserMedia` 返回带可观察 `stop` 的假 track；在 `effectScope` 或挂载一个最小组件里调用）：**
- 正常 start → isActive true；stop → track.stop 被调用、isActive false。
- start 未 resolve 时 scope 被 dispose → resolve 后该流的 track 被 stop，isActive 保持 false。
- 连续两次 flip（第一次还没 resolve）→ 最终只剩最后一个流活着，前一个流的 track 都被 stop。
- 非安全上下文 → start 返回 false，error 非空，未调用 getUserMedia。
- torch：capabilities 含 torch → torchSupported true；setTorch(true) 调用 applyConstraints。
- 现有 `frontend/src/components/customer/customer.spec.ts` 里 VisionSearch 相关用例保持通过。

**验收：** `cd frontend && npx vitest run`，`npm --prefix frontend run lint`，`npm --prefix frontend run typecheck`。
提交：`♻️ refactor: 摄像头取流抽成 useCamera，修复取流中途卸载 / 连点翻转时 track 泄漏`

---

### Task 4: 扫码引擎、扫码面板与点单页入口

依赖 Task 1–3 已合并。

**Files:**
- Modify: `frontend/package.json`（dependencies 加 `"barcode-detector": "3.2.2"` 与 `"zxing-wasm": "3.1.3"`——两者都**精确钉版本**，且 zxing-wasm 必须等于 barcode-detector 自己依赖的那个版本（wasm 与 JS 胶水必须同版本；`npm ls zxing-wasm` 只能出现一个版本）；`npm --prefix frontend install` 更新 lockfile）
- Create: `frontend/src/composables/useBarcodeScanner.ts`
- Create: `frontend/src/utils/barcodeEngine.ts`（引擎加载：原生优先，否则 ponyfill + 本地 wasm）
- Create: `frontend/src/utils/scanBeep.ts`（Web Audio 短促提示音，成功/失败两种音高）
- Create: `frontend/src/components/customer/BarcodeScanPanel.vue`
- Test: `frontend/src/composables/useBarcodeScanner.spec.ts`、`frontend/src/components/customer/barcodeScan.spec.ts`、`frontend/src/utils/barcodeEngine.node.spec.ts`
- Modify: `frontend/src/views/CustomerView.vue`（入口布局、第三种模式、吸引屏）
- Modify: `frontend/src/components/customer/customer.spec.ts`（入口布局变化后的既有断言；`CustomerView 拍照识别入口` 三个用例保留语义）

**Interfaces:**
- Consumes：`matchBarcode`（Task 2）、`useCamera`（Task 3）、`Schemas['ProductEventProduct']['barcode']`（Task 1）。
- Produces：

```ts
// barcodeEngine.ts
export const SCAN_FORMATS = ['ean_13','ean_8','upc_a','upc_e','code_128','code_39','qr_code'] as const
export interface Detector { detect(source: ImageBitmapSource): Promise<{ rawValue: string }[]> }
export function loadDetector(): Promise<Detector>   // 结果缓存为单例

// useBarcodeScanner.ts
export function useBarcodeScanner(opts: {
  video: Ref<HTMLVideoElement | null>
  roi: () => { x: number; y: number; w: number; h: number }  // 视频像素坐标
  onCode: (code: string) => void
  intervalMs?: number     // 默认 200
  cooldownMs?: number     // 默认 1500
  goneFrames?: number     // 默认 3：连续这么多帧没看到该码，视为已离开画面
}): { running: Readonly<Ref<boolean>>; loading: Readonly<Ref<boolean>>; error: Ref<string>;
      start(): Promise<void>; pause(): void; resume(): void; stop(): void }

// BarcodeScanPanel.vue
props: { products: Schemas['ProductEventProduct'][] }
emits: { add: [product: Schemas['ProductEventProduct']] }   // 父组件负责售罄以外的加购
```

**引擎（`barcodeEngine.ts`）：**
- `globalThis.BarcodeDetector` 存在且 `await BarcodeDetector.getSupportedFormats()` 覆盖 `ean_13` 与 `qr_code` → 用原生。
- 否则 `await import('barcode-detector/ponyfill')`，先 `prepareZXingModule({ overrides: { locateFile: (path, prefix) => path.endsWith('.wasm') ? wasmUrl : prefix + path } })`，`wasmUrl` 来自 `import wasmUrl from 'zxing-wasm/reader/zxing_reader.wasm?url'`（zxing-wasm 3.1.3 的 exports 已包含 `./reader/zxing_reader.wasm`；`npm run build` 后 `dist/assets/` 里必须有 `.wasm` 文件）。再 `new BarcodeDetector({ formats: [...SCAN_FORMATS] })`。
- 整个模块只在扫码面板首次打开时通过动态 `import()` 引入。

**扫描器（`useBarcodeScanner.ts`）：**
- 每 `intervalMs` 把 `roi` 区域画到离屏 canvas（长边缩到 ≤ 640px）再 `detect`；上一次 detect 未完成时跳过本拍。
- 防重复：记住「上一个上报的码」与上报时间、以及「连续未见帧数」。同码再次出现时：若期间连续未见 ≥ `goneFrames` 或距上次上报 ≥ `cooldownMs` → 再上报，否则忽略。不同的码立即上报。
- `pause()` 期间不 detect、不上报；`resume()` 后防重复状态清零。
- 卸载时停止定时器。

**面板（`BarcodeScanPanel.vue`）：**
- 用 `useCamera({ facing: 'environment' })`；挂载即 `start()`；提供「翻转」「补光」（仅 torchSupported 时显示）「返回商品列表」（emit `close`，由父组件切回列表）。
- 横向长条取景框：宽为视口宽 80%，高为宽的 40%，居中；`roi` 按 `object-fit: cover` 的实际裁切换算成视频像素（参考 VisionSearch 的 `captureFrame` 换算）。
- `onCode(code)` → `matchBarcode(code, products)`：
  - `hit` 1 件：`onsite_qty <= 0` → 闪红 + 提示行「{name} 已售罄」+ 失败音；否则 emit `add`、闪绿、成功音、最近记录顶部插入「+1 {name} ¥{价格}」（保留最近 5 条；价格用现有金额格式化工具，同 CustomerView 用法）。
  - `hit` 多件：`pause()`，弹 `AppModal` 列出候选（名称 + product_code），点一件等同上面的单件处理；关闭或选完 `resume()`。
  - `not_found`：闪红 + 「未找到条码 {code}」+ 失败音。
  - `ignored`：无任何反应。
- 除多件选择外不弹任何对话框（不调用 `fb.alert`）。
- 摄像头不可用（`useCamera.error` 非空）时显示错误文案与「返回商品列表」。
- 页面进入后台（`document.visibilitychange` → hidden）时 `stop()` 摄像头与扫描；回到前台且面板仍在时重新 start。

**点单页（`CustomerView.vue`）：**
- 把 `isVisionMode: boolean` 换成 `mode: 'list' | 'vision' | 'scan'`（所有读写处同步改，包括闲置计时器里回到 `'list'`、`refreshVisionAvailability` 里不可用时回到 `'list'`）。
- 工具栏两处 mode-toggle：「商品列表」是主按钮样式；旁边两个小一级的次级按钮「📷 拍照识别」「▦ 扫码」（尺寸、字号小于主按钮；拍照识别仍按 `visionAvailable` 置灰）。在 vision / scan 模式下「商品列表」按钮即返回。
- 扫码按钮置灰条件：`!window.isSecureContext || !navigator.mediaDevices?.getUserMedia`，title 说明原因「需要 HTTPS 连接才能使用摄像头」。
- `mode === 'scan'` 时渲染 `BarcodeScanPanel`（用 `defineAsyncComponent` 懒加载），`@add` 调 `store.addToCart(product)`，`@close` 回 `'list'`。购物车在该模式下保持可见（同 vision 模式的布局处理）。
- 吸引屏：大按钮只留「开始点单」（进入 `'list'`）；其下一行小字链接「拍照识别」「扫码」（拍照识别不可用时置灰且不可点）。

**必测：**
- `useBarcodeScanner`（mock `loadDetector` 返回可编程的 detect 结果、fake timers）：同码连续 10 帧只上报 1 次；消失 ≥3 帧后再出现 → 再上报；持续可见超过 1500ms → 再上报；不同码立即上报；pause 期间不上报；detect 未返回时不重入。
- `barcodeScan.spec.ts`（stub `useCamera`/`useBarcodeScanner`，直接调用 onCode）：命中有货 → emit add + 最近记录出现；售罄 → 不 emit、出现「已售罄」；未找到 → 出现「未找到条码」且没有弹窗；多件 → 出现选择弹窗且扫描器 pause，选一件后 emit add 并 resume；ignored → 无变化。
- `barcodeEngine`：无原生 BarcodeDetector 时调用了 `prepareZXingModule`，且其 `locateFile('zxing_reader.wasm', 'x/')` 返回值不以 `http` 开头。
- `barcodeEngine.node.spec.ts`（文件顶部 `// @vitest-environment node`）：用 `zxing-wasm/writer`（或 full）生成 EAN-13 `4901234567894`、UPC-A `036000291452`、Code128 `P001`、QR `P001` 的 PNG，再用 `zxing-wasm/reader` 的 `prepareZXingModule({ overrides: { wasmBinary } })`（`wasmBinary` 用 `fs` 读取 `node_modules/zxing-wasm/dist/reader/zxing_reader.wasm`）+ `readBarcodes` 解出原值。测试中禁止网络：`vi.stubGlobal('fetch', () => { throw new Error('no network') })`。
- `customer.spec.ts`：入口改版后，未就绪时拍照识别次级按钮与吸引屏链接置灰；扫码按钮在 jsdom（非安全上下文）下置灰——必要时用 `vi.stubGlobal('isSecureContext', true)` 与 mediaDevices mock 覆盖可用分支。

**验收：**
```bash
npm --prefix frontend install
npm --prefix frontend run typecheck && npm --prefix frontend run lint
cd frontend && npx vitest run
npm --prefix frontend run build && ls frontend/dist/assets/*.wasm   # 必须存在
grep -l "cdn.jsdelivr.net" frontend/dist/assets/*.js || true         # 允许出现在库的默认值里，但 locateFile 已覆盖——在报告里说明
```
提交：`✨ feat: 点单页扫码——摄像头连续扫描条码加入购物车`

---

### Task 5: 扫码枪与表单「扫码填入」

依赖 Task 4。

**Files:**
- Create: `frontend/src/composables/useScanGun.ts`
- Test: `frontend/src/composables/useScanGun.spec.ts`
- Modify: `frontend/src/views/CustomerView.vue`（在任何 mode 下启用扫码枪；命中走与面板相同的处理）
- Modify: `frontend/src/components/customer/BarcodeScanPanel.vue`（新增 `props.single?: boolean`：单次模式——扫到第一个非 ignored 的码即 emit `code` 并停止，不做匹配、不加购）
- Modify: `frontend/src/components/product/CreateMasterProductForm.vue`、`EditMasterProductModal.vue`（条码输入框旁「扫码填入」按钮 → `AppModal` 里放单次模式面板，结果写入输入框）
- 抽取：Task 4 面板里「命中后的处理（售罄判断、加购、提示）」若 CustomerView 也要复用，抽成 `frontend/src/composables/useScanResultHandler.ts`；提示行在扫码面板外时用 `useFeedback` 的轻提示（`fb.success` / `fb.error`，非模态）。

**Interfaces:**

```ts
export function useScanGun(opts: {
  onCode: (code: string) => void
  maxIntervalMs?: number   // 默认 50：相邻按键间隔上限
  minLength?: number       // 默认 4
  enabled?: Ref<boolean>
}): void   // 在 window 上监听 keydown，onScopeDispose 时移除
```

**规则：**
- 事件目标是 `input` / `textarea` / `select` / `contenteditable` 时完全不处理。
- 可打印单字符按键累加进缓冲；相邻间隔 > `maxIntervalMs` 时缓冲重置为当前字符。
- `Enter` 时缓冲长度 ≥ `minLength` 且整个缓冲都在间隔内输入 → `onCode(buffer)` 并 `preventDefault()`；否则丢弃缓冲、不拦截。
- CustomerView：吸引屏显示时扫码枪命中 → 先 `dismissAttractScreen()` 再加购。

**必测：**快速输入 `4901234567894` + Enter → onCode 一次；人手速度（每键 150ms）输入同样内容 + Enter → 不触发；焦点在 input 内快速输入 + Enter → 不触发且不 preventDefault；长度 3 → 不触发；卸载后不再监听。单次模式面板：扫到码 → emit `code` 并停止扫描。编辑表单：「扫码填入」回填输入框（stub 面板）。

**验收：** 同 Task 4 的前端验收命令。
提交：`✨ feat: 点单页支持扫码枪；商品表单可扫码填入条码`

---

## 执行后记（2026-09-26）

**执行方式**：dsh-flash 实现、Claude 审查。批次 A（Task 1–3）在三个 worktree 里并行跑，批次 B（Task 4→5）交给同一个 worker 串行做；最后由 Opus 做全分支终审，发现的问题打包成一次修复。

**计划外的改动与裁定：**

- **防重复规则**：spec §5.2 写的是「离开画面**或**距上次超过 1.5 秒」再算一次。端到端实测发现，码一直停在画面里，每 1.5 秒就会重复加一件，和 spec 自己的意图相反。已改为：码先离开画面（连续 ≥3 帧没看到），**并且**距上次上报 ≥1.5 秒，才算下一次。
- **按码分别去重**：终审发现，日本书的 ISBN 和价格码（191/192）经常同时出现在取景框里。只记「上一个码」的话，两个码会互相刷新，每帧都重复上报。现在按每个码分别记录状态。控制者起初判断这种情况罕见并搁置，这个判断是错的。
- **库存占满**：购物车里某商品的数量已达现场库存时，扫码按失败处理，闪红提示，不调用 `addToCart`（它自己会弹一个模态框，会卡住连续扫描）。
- **扫码枪**：忽略 Shift 等修饰键（扫码枪输出大写字母前会发 Shift）。结算确认、付款弹窗、成功屏以及多件选择弹窗打开期间停用扫码枪。
- **结算期间卸载扫码面板**：结算确认、付款弹窗、成功屏打开期间，扫码面板直接卸载，摄像头随之释放。
- **`useCamera.flip()`**：改为先停旧流再开新流。很多 Android 设备不能同时打开两个摄像头。
- **不检测设备有没有摄像头**：spec §5.1 里用 `enumerateDevices` 判断的做法没采用，只看是否为安全上下文、是否有 `getUserMedia`。
- **沙箱限制**：dsh-flash 的沙箱只允许写 `--cwd` 目录内部，所以 worker 在 worktree 里无法提交，全部由控制者原样代为提交。

**验证**：cargo test 354 个、vitest 437 个全过，两种 feature 组合的 clippy 都干净。另外在真实后端加 Chromium 假摄像头上做了端到端：往假摄像头喂 Code128 图片，WASM 引擎约 0.5 秒识别；ISBN 和价格码同框时只加一件；全程没有外部网络请求；扫码面板和 wasm 都是按需加载的独立 chunk。

**还要上真机验的**：
- 安卓平板后置摄像头：EAN-13 和 QR 各扫一次；断网再扫一遍。
- 安卓设备上两个摄像头的「翻转」，拍照识别和扫码都要试。
- Windows 笔记本摄像头扫 1D 条码。
- iPhone Safari 经局域网访问。
- 平板转屏后，取景框和实际解码区域是否还对齐。
- 表单「扫码填入」弹窗里的取景区高度。
- 中文 Windows 开着输入法时，扫码枪会失效（按键以 `Process` 形式到达，是原有的问题）。
