# Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current "open GitHub releases page" update flow on Windows with a real download → rsign signature verify → install → restart cycle, driven by `tauri-plugin-updater` + `tauri-plugin-process`. Android keeps the manual fallback.

**Architecture:** Register the two Tauri plugins (updater already in `Cargo.toml`, process needs to be added), wire capabilities, add the updater config block in `tauri.conf.json` pointing at a `latest.json` asset hosted on each GitHub release. Frontend composable `useUpdateCheck.js` swaps its GitHub-API polling for `plugin-updater`'s `check()/downloadAndInstall()`; `UpdateModal.vue` gains a progress bar + a restart confirmation. On platforms where updater isn't available (Android), the composable branches back to the old `goToDownload()` behaviour.

**Tech Stack:** Rust (Tauri 2), Vue 3, `@tauri-apps/plugin-updater@2`, `@tauri-apps/plugin-process@2`, GitHub Releases as CDN, rsign signing (key already in `secret-key.txt`).

**Testing approach:** No unit-test infra is being added (per spec decision — scope too small, plugin boundary is library-tested upstream). Verification per task: `cargo check`, `npm run build`, plus a final end-to-end smoke test documented in Task 8.

**Spec:** `docs/superpowers/specs/2026-04-24-auto-update-design.md`

---

### Task 1: Add `tauri-plugin-process` dep and register both plugins

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `tauri-plugin-process` next to the existing `tauri-plugin-updater` platform-scoped block)
- Modify: `src-tauri/src/lib.rs:52-57` (uncomment + add process plugin, cfg-gated)

- [ ] **Step 1: Add `tauri-plugin-process` to Cargo.toml**

Edit `src-tauri/Cargo.toml`. Find line 79–80:

```toml
[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]
tauri-plugin-updater = "2"
```

Replace with:

```toml
[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

- [ ] **Step 2: Register plugins in `lib.rs`**

Edit `src-tauri/src/lib.rs`. Find line 51-57:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        //.plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
```

Replace with:

```rust
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
```

Also find the chain continuation on line 58 (`.invoke_handler(...)`). Because we changed `tauri::Builder::default()...` from a chained expression to a `let`-bound `builder`, all subsequent `.method()` calls need to be on `builder` instead of being part of the chain. Replace the rest of the chain (lines 58 through the end of the chain — find the `.run(...)` call that ends the expression) by rewriting as:

```rust
    builder
        .invoke_handler(tauri::generate_handler![get_backend_url])
        .setup(|app| {
            // ... (keep existing setup body verbatim)
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
```

NOTE TO EXECUTOR: the `.setup(|app| { ... })` body is ~150 lines. DO NOT rewrite its body — only move the chain start from `tauri::Builder::default().plugin(...)...setup(...)` to `builder.setup(...)`. Use the Edit tool on the exact boundaries: the opening `tauri::Builder::default()` through the last `.plugin(...)` call (which becomes the `let mut builder = ...;` block), and leave everything from `.invoke_handler` onward unchanged except for prefixing it with `builder`.

- [ ] **Step 3: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: `Finished ... dev profile` with no errors. Warnings about unused `mut` on `builder` (if any) are fine — `cfg` branches use it.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "build: :package: wire up tauri-plugin-updater + tauri-plugin-process (desktop only)"
```

---

### Task 2: Extract public signing key and add updater config

**Files:**
- Create: `src-tauri/updater-key.pub` (gitignored later; the public key is also inlined in tauri.conf.json but keeping a file copy is convenient for tooling)
- Modify: `src-tauri/tauri.conf.json` (add `plugins.updater` block)
- Modify: `.gitignore` (ensure `secret-key.txt` stays out — probably already is)

- [ ] **Step 1: Determine if a corresponding `.pub` file already exists**

Run: `ls E:\Tauri\booth-tool\secret-key.txt*` (or `cmd /c dir /b E:\Tauri\booth-tool\secret-key*`)
Expected outcome — one of:
  a) Only `secret-key.txt` exists → go to Step 2 (regenerate pair).
  b) A `secret-key.txt.pub` or similar `.pub` file exists → skip Step 2, use that file directly, go to Step 3.

- [ ] **Step 2: (If .pub missing) Regenerate keypair**

The private `secret-key.txt` was generated without its `.pub` sibling ever being committed or stored, so there is no way to recover the public key from it without the original password and toolchain. Regenerate a fresh keypair. Since no release has yet been signed against the old key (the updater was never active), no signatures in the wild will be invalidated.

Run (in a fresh terminal; this is interactive — it will ask for a password):

```
cd E:\Tauri\booth-tool
npx @tauri-apps/cli signer generate -w src-tauri/updater-key.key
```

When prompted:
- Password: choose a strong password and **record it in your password manager**. You will need to set it in `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` every release build.

Produces:
- `src-tauri/updater-key.key` — new encrypted private key
- `src-tauri/updater-key.key.pub` — corresponding public key

Now replace the old `secret-key.txt`:

```
del E:\Tauri\booth-tool\secret-key.txt
move /Y E:\Tauri\booth-tool\src-tauri\updater-key.key E:\Tauri\booth-tool\secret-key.txt
move /Y E:\Tauri\booth-tool\src-tauri\updater-key.key.pub E:\Tauri\booth-tool\secret-key.txt.pub
```

(Keep the repo-root location so earlier mental model / tooling doesn't break.)

- [ ] **Step 3: Confirm `secret-key.txt` is in `.gitignore`**

Run: `findstr /i "secret-key" E:\Tauri\booth-tool\.gitignore`
Expected: at least one line matching. If not, append:

```
secret-key.txt
```

The `.pub` file IS committed (it's public, not secret).

- [ ] **Step 4: Read the public key value**

Run: `cmd /c type E:\Tauri\booth-tool\secret-key.txt.pub`

The file looks like:

```
untrusted comment: minisign public key XXXXXXXX
<BASE64_KEY_ON_ONE_LINE>
```

Copy only the base64 line (without the comment). This is the string you inline into `tauri.conf.json` — Tauri expects the raw public-key value on one line, NOT the full minisign-format file.

- [ ] **Step 5: Add updater config to `tauri.conf.json`**

Edit `src-tauri/tauri.conf.json`. Find the root object's last top-level key `"bundle": { ... }`. Insert before the closing `}` of the root object (after the `"bundle"` block):

```jsonc
  ,
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_THE_BASE64_KEY_FROM_STEP_4_HERE",
      "windows": {
        "installMode": "passive"
      }
    }
  }
```

Replace `PASTE_THE_BASE64_KEY_FROM_STEP_4_HERE` with the actual base64 value from Step 4.

- [ ] **Step 6: Validate JSON + recompile**

Run: `cd src-tauri && cargo check`
Expected: Finished, no errors. The Tauri build script reads `tauri.conf.json` and will fail with a schema error if JSON is malformed or the `pubkey` shape is wrong.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/tauri.conf.json secret-key.txt.pub .gitignore
git commit -m "build: :lock: add updater signing pubkey and endpoint config"
```

(If Step 2 was NOT run, omit `secret-key.txt.pub` from the add list — it already existed as committed.)

---

### Task 3: Enable updater + process capabilities

**Files:**
- Modify: `src-tauri/capabilities/default.json` (add `updater:default`, `process:default`)

- [ ] **Step 1: Add permissions**

Edit `src-tauri/capabilities/default.json`. Find the `"permissions": [ ... ]` array. Inside the array, after `"core:event:default",` (line 9 in current file), add two new entries:

```jsonc
    "core:default",
    "core:event:default",
    "updater:default",
    "process:default",
    "shell:allow-open",
    /* ... (rest of existing permissions unchanged) ... */
```

i.e. two new string entries `"updater:default"` and `"process:default"` inserted after `"core:event:default",`.

- [ ] **Step 2: Verify**

Run: `cd src-tauri && cargo check`
Expected: Finished, no errors. Tauri's build-time capability checker will fail if the permission IDs don't match what the plugins declare.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/capabilities/default.json
git commit -m "build: :key: grant updater + process capabilities to main window"
```

---

### Task 4: Install frontend plugin packages

**Files:**
- Modify: `frontend/package.json`
- Modify: `frontend/package-lock.json` (auto-written by npm)

- [ ] **Step 1: Install**

Run: `cd frontend && npm install @tauri-apps/plugin-updater @tauri-apps/plugin-process`
Expected: both packages resolve at version `^2.x.x` and appear in `dependencies` in `package.json`.

- [ ] **Step 2: Verify build still green**

Run: `cd frontend && npm run build`
Expected: `✓ built in X.XXs`. Just adding deps shouldn't break anything.

- [ ] **Step 3: Commit**

```bash
git add frontend/package.json frontend/package-lock.json
git commit -m "deps: :arrow_up: add @tauri-apps/plugin-updater + plugin-process"
```

---

### Task 5: Rewrite `useUpdateCheck.js` composable

**Files:**
- Modify: `frontend/src/composables/useUpdateCheck.js` (full rewrite, keep backward-compat API surface used by `UpdateModal.vue`)

- [ ] **Step 1: Replace composable body**

Overwrite `frontend/src/composables/useUpdateCheck.js` with:

```js
import { ref } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import { copyLink } from '@/services/clipboard';

const GITHUB_USER = 'Academy-of-Boundary-Landscape';
const GITHUB_REPO = 'ABL-BoothApp';
const DOWNLOAD_PAGE_URL = `https://github.com/${GITHUB_USER}/${GITHUB_REPO}/releases/latest`;

// Lazy load platform modules — they are desktop-only and will throw on Android.
async function loadDesktopModules() {
  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ]);
  return { check, relaunch };
}

async function detectPlatform() {
  try {
    const { platform } = await import('@tauri-apps/plugin-os');
    return await platform();
  } catch {
    return 'web';
  }
}

export function useUpdateCheck() {
  const loading = ref(false);
  const error = ref(null);
  const hasUpdate = ref(false);
  const currentVersion = ref('');
  const latestVersion = ref('');
  const releaseNote = ref('');
  const releaseDate = ref('');
  const platform = ref('');

  // 内部持有的 Update 对象，供 downloadAndInstall 使用（checkUpdate 成功时设置）
  let pendingUpdate = null;

  // 新增：下载 / 安装状态
  const isDownloading = ref(false);
  const downloadProgress = ref({ downloaded: 0, total: 0, percent: 0 });
  const isInstalled = ref(false); // true 表示已装完，可重启

  const isTauri = () => typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;

  const canAutoUpdate = async () => {
    if (!isTauri()) return false;
    const p = await detectPlatform();
    platform.value = p;
    // Tauri updater 在桌面平台可用；android/ios 不走这条路
    return p === 'windows' || p === 'macos' || p === 'linux';
  };

  const checkUpdate = async () => {
    if (!isTauri()) {
      error.value = '请在 App 环境中运行';
      return;
    }

    loading.value = true;
    error.value = null;
    hasUpdate.value = false;
    pendingUpdate = null;

    try {
      currentVersion.value = await getVersion();

      const auto = await canAutoUpdate();
      if (!auto) {
        // Android/iOS 走 GitHub API 轻量检查，复用原逻辑
        await checkViaGithubApi();
        return;
      }

      const { check } = await loadDesktopModules();
      const upd = await check();

      if (upd === null) {
        hasUpdate.value = false;
        latestVersion.value = currentVersion.value;
        return;
      }

      pendingUpdate = upd;
      hasUpdate.value = true;
      latestVersion.value = upd.version;
      releaseNote.value = upd.body || '暂无更新日志';
      releaseDate.value = upd.date || '';
    } catch (e) {
      console.error('更新检查出错:', e);
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  };

  // Android fallback: 沿用旧的 GitHub API 查版本逻辑（仅展示，不下载）
  const checkViaGithubApi = async () => {
    const { fetch } = await import('@tauri-apps/plugin-http');
    const url = `https://api.github.com/repos/${GITHUB_USER}/${GITHUB_REPO}/releases/latest`;
    const response = await fetch(url, {
      method: 'GET',
      headers: { 'User-Agent': 'Tauri-App-Updater' },
    });
    if (!response.ok) {
      if (response.status === 403) throw new Error('检查过于频繁，请稍后再试');
      throw new Error(`请求失败: ${response.status} ${response.statusText}`);
    }
    const data = await response.json();
    const remoteTag = (data.tag_name || '').replace(/^v/, '');
    latestVersion.value = remoteTag;
    releaseNote.value = data.body || '暂无更新日志';
    releaseDate.value = data.published_at || '';
    hasUpdate.value = compareVersions(remoteTag, currentVersion.value) === 1;
  };

  const compareVersions = (v1, v2) => {
    const a = v1.split('.').map(Number);
    const b = v2.split('.').map(Number);
    const len = Math.max(a.length, b.length);
    for (let i = 0; i < len; i++) {
      const x = a[i] || 0;
      const y = b[i] || 0;
      if (x > y) return 1;
      if (x < y) return -1;
    }
    return 0;
  };

  const downloadAndInstall = async () => {
    if (!pendingUpdate) {
      error.value = '没有待下载的更新';
      return;
    }
    isDownloading.value = true;
    error.value = null;
    downloadProgress.value = { downloaded: 0, total: 0, percent: 0 };
    isInstalled.value = false;

    try {
      let totalBytes = 0;
      let downloadedBytes = 0;

      await pendingUpdate.downloadAndInstall((ev) => {
        // ev.event: 'Started' | 'Progress' | 'Finished'
        switch (ev.event) {
          case 'Started':
            totalBytes = ev.data.contentLength || 0;
            downloadedBytes = 0;
            downloadProgress.value = {
              downloaded: 0,
              total: totalBytes,
              percent: 0,
            };
            break;
          case 'Progress':
            downloadedBytes += ev.data.chunkLength || 0;
            downloadProgress.value = {
              downloaded: downloadedBytes,
              total: totalBytes,
              percent: totalBytes > 0
                ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))
                : 0,
            };
            break;
          case 'Finished':
            downloadProgress.value = {
              downloaded: totalBytes,
              total: totalBytes,
              percent: 100,
            };
            break;
        }
      });
      isInstalled.value = true;
    } catch (e) {
      console.error('下载/安装失败:', e);
      error.value = e instanceof Error ? e.message : String(e);
      isInstalled.value = false;
    } finally {
      isDownloading.value = false;
    }
  };

  const restartApp = async () => {
    const { relaunch } = await loadDesktopModules();
    await relaunch();
  };

  const goToDownload = async () => {
    try {
      await copyLink(DOWNLOAD_PAGE_URL);
    } catch (e) {
      console.error('复制链接失败:', e);
    }
    if (isTauri()) {
      await open(DOWNLOAD_PAGE_URL);
    } else {
      window.open(DOWNLOAD_PAGE_URL, '_blank');
    }
  };

  return {
    loading,
    error,
    hasUpdate,
    currentVersion,
    latestVersion,
    releaseNote,
    releaseDate,
    platform,

    isDownloading,
    downloadProgress,
    isInstalled,

    checkUpdate,
    downloadAndInstall,
    restartApp,
    goToDownload,
    canAutoUpdate,
  };
}
```

- [ ] **Step 2: Verify build**

Run: `cd frontend && npm run build`
Expected: built cleanly. If vite complains about the dynamic imports, double-check the plugin packages were installed in Task 4.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/composables/useUpdateCheck.js
git commit -m "feat: :sparkles: wire useUpdateCheck to tauri-plugin-updater"
```

---

### Task 6: Upgrade `UpdateModal.vue` with progress bar + restart flow

**Files:**
- Modify: `frontend/src/components/shared/UpdateModal.vue`

- [ ] **Step 1: Replace template "state 4" (发现新版本) and actions**

Open `frontend/src/components/shared/UpdateModal.vue`. Find the block starting with `<!-- 状态 4: 发现新版本 -->` (current line 52) through `</div>` that closes `update-content` (current line 77). Replace with:

```vue
    <!-- 状态 4: 发现新版本 -->
    <div v-else class="update-content">
      <div class="header-section">
        <n-tag type="success" size="large">新版本 v{{ latestVersion }}</n-tag>
        <span class="date">{{ formatDate(releaseDate) }}</span>
      </div>

      <div class="current-ver-tip">
        当前版本: v{{ currentVersion }}
      </div>

      <n-divider title-placement="left" style="margin: 12px 0;">更新内容</n-divider>

      <n-scrollbar style="max-height: 200px" class="log-scroll">
        <div class="release-note">{{ releaseNote }}</div>
      </n-scrollbar>

      <!-- 下载进度 -->
      <div v-if="isDownloading" class="progress-section">
        <n-progress
          type="line"
          :percentage="downloadProgress.percent"
          :indicator-placement="'inside'"
        />
        <p class="progress-text">
          正在下载 {{ formatBytes(downloadProgress.downloaded) }} /
          {{ formatBytes(downloadProgress.total) }}
        </p>
      </div>

      <!-- 安装完成提示 -->
      <div v-else-if="isInstalled" class="installed-hint">
        <n-alert type="success" :bordered="false">
          新版本已安装完成。点击「立即重启」以完成更新。
        </n-alert>
      </div>

      <div class="actions">
        <!-- 安装完成后：展示重启按钮 -->
        <template v-if="isInstalled">
          <n-button @click="close" ghost>稍后重启</n-button>
          <n-button type="primary" @click="confirmRestart">立即重启</n-button>
        </template>
        <!-- 下载中：禁用操作按钮，防止中断 -->
        <template v-else-if="isDownloading">
          <n-button disabled>下载中... {{ downloadProgress.percent }}%</n-button>
        </template>
        <!-- 初始状态：展示下载 / 取消 -->
        <template v-else>
          <n-button @click="close" ghost>暂不更新</n-button>
          <n-button
            v-if="canAuto"
            type="primary"
            @click="handleAutoInstall"
          >
            下载并安装
          </n-button>
          <n-button
            v-else
            type="primary"
            @click="handleDownload"
          >
            前往下载
          </n-button>
        </template>
      </div>
    </div>
```

- [ ] **Step 2: Update `<script setup>` imports and logic**

Replace the entire `<script setup lang="ts">` block (current lines 81–137) with:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useUpdateCheck } from '@/composables/useUpdateCheck';
import {
  NModal, NSpin, NResult, NButton, NTag, NDivider, NScrollbar, NAlert, NProgress,
  useDialog,
} from 'naive-ui';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['update:show']);

const isTauriEnv = ref(typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__ !== undefined);
const dialog = useDialog();

const showModal = computed({
  get: () => props.show,
  set: (val) => emit('update:show', val),
});

const {
  loading,
  error,
  hasUpdate,
  currentVersion,
  latestVersion,
  releaseNote,
  releaseDate,

  isDownloading,
  downloadProgress,
  isInstalled,

  checkUpdate,
  downloadAndInstall,
  restartApp,
  goToDownload,
  canAutoUpdate,
} = useUpdateCheck();

const canAuto = ref(false);

onMounted(async () => {
  if (isTauriEnv.value) {
    canAuto.value = await canAutoUpdate();
  }
});

const handleEnter = () => {
  if (isTauriEnv.value) {
    checkUpdate();
  }
};

const retry = () => {
  if (isTauriEnv.value) {
    checkUpdate();
  }
};

const close = () => {
  showModal.value = false;
};

const handleDownload = () => {
  goToDownload();
};

const handleAutoInstall = async () => {
  await downloadAndInstall();
  // downloadAndInstall 失败时 error.value 会被设置；成功时 isInstalled 变 true。
  // 这里不自动跳转，让用户看到"已安装"提示再点重启。
};

const confirmRestart = () => {
  dialog.warning({
    title: '即将重启摊盒',
    content: '重启会关闭应用以完成安装。请确认当前没有未保存的订单或编辑。继续吗？',
    positiveText: '确认重启',
    negativeText: '再等等',
    onPositiveClick: async () => {
      await restartApp();
    },
  });
};

const formatDate = (dateStr: string) => {
  if (!dateStr) return '';
  return new Date(dateStr).toLocaleDateString();
};

const formatBytes = (bytes: number) => {
  if (!bytes || bytes < 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i += 1;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
};
</script>
```

- [ ] **Step 3: Add progress styles**

Append to the existing `<style scoped>` block (before the final `</style>`):

```css
.progress-section {
  margin-top: 1rem;
}
.progress-text {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-top: 0.5rem;
  text-align: center;
}
.installed-hint {
  margin-top: 1rem;
}
```

- [ ] **Step 4: Verify build**

Run: `cd frontend && npm run build`
Expected: built cleanly. TypeScript check of `<script setup lang="ts">` will catch any typos in the imports.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/shared/UpdateModal.vue
git commit -m "feat: :sparkles: UpdateModal — download progress + restart confirmation"
```

---

### Task 7: Write user-facing docs

**Files:**
- Create: `docs/guide/auto-update.md`
- Modify: `docs/BUILD.md` (append release-process section)

- [ ] **Step 1: Create user guide**

Create `docs/guide/auto-update.md` with:

```markdown
# 自动更新

摊盒 v1.2.0 起支持一键更新。不再需要打开 GitHub 页面手动下载安装。

## 如何检查和更新

1. 打开「**控制台**」
2. 找到「**检查更新**」入口
3. 摊盒会访问 GitHub 询问是否有新版本
4. 若有新版本，会展示更新日志；点击「**下载并安装**」
5. 下载完成后会提示「**立即重启**」或「**稍后重启**」
   - 重启前请确认没有未保存的订单或编辑
6. 重启后，新版本就生效了

## 什么时候更新不会生效

- **Android 版**目前不支持一键更新，仍需从发布页下载 APK 手动安装
- **无网络**时会报告「网络错误」，连上网络再重试
- **更新包校验失败**说明下载内容不完整或被篡改 —— 请从官方 GitHub 页面手动下载

## 从 v1.1.0 及以下升级到 v1.2.0

因为 v1.1.0 及更早版本没有「下载并安装」功能，**第一次升级到 v1.2.0 仍需手动下载**。从 v1.2.0 起才能真正自动更新。

## 隐私

- 摊盒只在你主动点「检查更新」时联网
- 除了 GitHub（获取版本号和安装包），不会访问其他服务器
- 不会上报任何使用数据
```

- [ ] **Step 2: Update developer build docs**

Read the current `docs/BUILD.md` first to decide where to append. Then append this section at the end of the file:

```markdown

## 发布新版本（带自动更新）

从 v1.2.0 开始，客户端会自动从 GitHub Releases 拉取 `latest.json` 判断更新。发布流程新增了签名和清单步骤。

### 准备（一次性）

- 公钥已在 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` 字段
- 私钥在仓库根的 `secret-key.txt`（已 gitignore）
- 请把**私钥密码**（生成时设置的）记在密码管理器里

### 每次发布

1. 更新版本号：
   - `src-tauri/tauri.conf.json` → `"version"`
   - `frontend/package.json` → `"version"`
   - 写 `CHANGELOG-v1.x.md`

2. 设置签名环境变量（PowerShell）：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content E:\Tauri\booth-tool\secret-key.txt -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<你的私钥密码>"
```

3. 构建：

```
cd E:\Tauri\booth-tool\frontend
npm run build

cd E:\Tauri\booth-tool
npm run tauri build
```

产出物在 `src-tauri/target/release/bundle/nsis/`：

- `摊盒_x.y.z_x64-setup.exe` — 安装器
- `摊盒_x.y.z_x64-setup.exe.sig` — rsign 签名文件

4. 生成 `latest.json`：

```json
{
  "version": "1.2.0",
  "notes": "摊盒 1.2.0 —— 支持一键自动更新",
  "pub_date": "2026-05-01T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<将 .sig 文件内容一行粘贴到这里>",
      "url": "https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases/download/v1.2.0/摊盒_1.2.0_x64-setup.exe"
    }
  }
}
```

- `signature` 字段是 `.sig` 文件的**完整内容**（包含 `untrusted comment:` 头那几行），但作为 JSON 字符串需要把换行编码为 `\n`。PowerShell 生成：

```powershell
(Get-Content "src-tauri/target/release/bundle/nsis/摊盒_1.2.0_x64-setup.exe.sig" -Raw) -replace "`r`n", "\n"
```

- `url` 必须是 GitHub release 上传后的最终下载链接。先上传 `.exe`，然后从 release 页面拷贝链接回来填入。

5. 在 GitHub 上：

- 创建 tag `v1.2.0`
- 创建 Release，粘贴 changelog
- 上传三个文件作为 asset：`摊盒_1.2.0_x64-setup.exe`、`摊盒_1.2.0_x64-setup.exe.sig`、`latest.json`
- Publish

6. 验证：

- 在一台装有旧版摊盒的机器（或一台没装的机器，先手装旧版）点「检查更新」
- 确认能看到 1.2.0、能下载、下载进度正确、重启后版本真的变了

### 常见问题

- **"invalid signature" 错误**：`.sig` 文件和 `.exe` 不匹配，或上传时顺序错。重新构建 + 重新上传。
- **"No version available" 错误**：`latest.json` 没传或名字不是 `latest.json`。
- **中文文件名下载后变 `???`**：GitHub Release 有时会对中文文件名 URL-encode。用英文文件名（如 `BoothTool_1.2.0_x64-setup.exe`）避开这个问题。需要同时改 `tauri.conf.json` 里 `productName` 的导出策略，或在 release 上传时重命名再填到 `latest.json` 里。
```

- [ ] **Step 3: Commit**

```bash
git add docs/guide/auto-update.md docs/BUILD.md
git commit -m "docs: :memo: user guide + release process for auto-update"
```

---

### Task 8: End-to-end smoke test protocol (manual)

**Files:** none — this is a test script to run, not code to write.

- [ ] **Step 1: Install "old" version**

- Build and install a locally-generated v1.1.99 (or whatever < bump target): bump `tauri.conf.json` to `1.1.99`, build NSIS installer, run it to install.
- Launch. Confirm the version displayed in About is `1.1.99`.

- [ ] **Step 2: Publish "new" version**

- On the same branch, bump `tauri.conf.json` to a higher version e.g. `1.1.100`, build NSIS installer + .sig.
- Generate a valid `latest.json` per Task 7 Step 2's recipe.
- Host the three files somewhere accessible. Quickest: create a draft GitHub release in a personal repo and publish it; update `tauri.conf.json`'s `endpoints` to point there temporarily. Or run a local HTTPS server with `mkcert` and the three files.

- [ ] **Step 3: Click "检查更新" in the installed v1.1.99**

Expected:
- Modal shows "新版本 v1.1.100"
- Changelog displays
- Button "下载并安装" is visible (because Windows auto-update path)

- [ ] **Step 4: Click "下载并安装"**

Expected:
- Progress bar appears and advances from 0% → 100%
- Bytes counter updates (e.g. `4.2 MB / 12.8 MB`)
- On success: button region switches to "立即重启" + "稍后重启"

- [ ] **Step 5: Click "立即重启"**

Expected:
- Confirmation dialog appears
- Clicking "确认重启" → app closes, installer runs in passive mode, app relaunches as v1.1.100
- Clicking "再等等" → dialog dismisses, main modal stays in installed state

- [ ] **Step 6: Error paths**

- Break the signature: edit `latest.json` to corrupt the signature field → repeat Step 4 → expect error "InvalidSignature" or similar surfaced in the modal.
- Break the URL: point `url` at a 404 → repeat Step 4 → expect error.
- No network: disable WiFi → click "检查更新" → expect error "network error" / similar.

- [ ] **Step 7: Revert test-only config**

Reset `tauri.conf.json`'s endpoints back to the production GitHub URL before merging.

- [ ] **Step 8: Commit (no-op if everything clean, otherwise any final polish commit)**

If issues found during smoke test, fix them in tiny follow-up commits referencing which step failed.

---

## Self-Review

**Spec coverage audit** (spec = `docs/superpowers/specs/2026-04-24-auto-update-design.md`):

- Covered platforms: Windows (Task 1–7) ✓, Android fallback (Task 5 — `canAutoUpdate`/`goToDownload` branch) ✓
- Plugins: updater (Task 1) ✓, process (Task 1) ✓
- Manifest location: latest.json on GitHub release (Task 2 config + Task 7 doc) ✓
- UX flow: manual trigger (Task 6 existing `handleEnter`) ✓; progress bar (Task 6) ✓; restart confirmation (Task 6 `confirmRestart`) ✓; Android fallback button (Task 6 `v-else`) ✓
- Error handling: network / sig / disk — covered by plugin + surfaced via `error` ref (Task 5) ✓; user-visible in modal (Task 6 `state 2`) ✓
- Release flow docs: Task 7 Step 2 ✓
- Smoke test protocol: Task 8 ✓
- Risk: v1.1.0 bridge upgrade (noted in Task 7 user guide) ✓
- Open issue: pubkey extraction (Task 2 Step 1–4, resolves with regenerate-if-missing branch) ✓

**Placeholder scan:** 
- `PASTE_THE_BASE64_KEY_FROM_STEP_4_HERE` in Task 2 Step 5 is explicitly instructed to be replaced in the same task — not a TODO, an explicit fill-in. Acceptable.
- No TBD / implement later / etc.

**Type/naming consistency:**
- `canAutoUpdate` defined in composable (Task 5) and destructured in UpdateModal (Task 6) ✓
- `isDownloading`, `downloadProgress`, `isInstalled` — refs defined in Task 5, consumed in Task 6 ✓
- `downloadAndInstall`, `restartApp` — same ✓

Plan is complete.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-24-auto-update.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using `executing-plans`, batch execution with checkpoints.

In a Ralph Loop context, inline execution is the natural fit (subagents can't persist across iterations anyway). Proceeding with inline execution.
