<template>
  <div>
    <header class="page-header">
      <div class="header-content">
        <h1>全局商品库</h1>
        <p>在这里管理所有可复用的商品模板。添加后,即可在展会中通过编号上架。</p>
      </div>
    </header>

    <!-- 导出/导入操作区 -->
    <section class="sync-section">
      <div class="section-header" @click="isSyncExpanded = !isSyncExpanded">
        <h2>制品数据一键导入/导出</h2>
        <n-button text class="toggle-btn">
          {{ isSyncExpanded ? '折叠' : '展开' }}
        </n-button>
      </div>
      <transition name="expand">
        <div v-show="isSyncExpanded" class="sync-container">
          <div class="section-content">
            <div class="info-box">
              <p class="info-text">
                <strong>💡 功能说明：</strong>您可以将当前的所有制品信息（包括图片）导出为一个 <code>.boothpack</code> 数据包，
                方便在其他设备上导入，或作为备份保存。导入时会自动覆盖或新增制品数据。
              </p>
              <p class="info-text info-warning">
                <strong>⚠️ 注意：</strong>导入操作会覆盖编号相同的制品，请谨慎操作。建议在导入前先导出当前数据作为备份。
              </p>
            </div>
            <div class="sync-controls">
              <n-button size="large" type="success" :loading="isExporting" @click="handleExport">
                <template #icon>
                  <span class="btn-icon">📦</span>
                </template>
                导出制品数据包
              </n-button>
              <n-button size="large" type="info" :loading="isImporting" @click="triggerImport">
                <template #icon>
                  <span class="btn-icon">📥</span>
                </template>
                导入制品数据包
              </n-button>
              <input
                ref="importFileInputRef"
                type="file"
                class="hidden-input"
                accept=".zip,.boothpack"
                @change="handleImportFile"
              />
            </div>
            <div
              class="drop-zone"
              :class="{ 'is-dragging': isDragging }"
              @dragenter.prevent="onDragEnter"
              @dragover.prevent="onDragOver"
              @dragleave.prevent="onDragLeave"
              @drop.prevent="onDrop"
            >
              <div class="drop-zone-content">
                <span class="drop-zone-icon">📂</span>
                <span class="drop-zone-text">将 .boothpack 或 .zip 文件拖拽到此处导入</span>
              </div>
            </div>
          </div>
        </div>
      </transition>
    </section>

    <main>
      <!-- 创建表单 -->
      <div class="form-section">
        <div class="section-header" @click="isFormExpanded = !isFormExpanded">
          <h2>添加新商品到仓库</h2>
          <n-button text class="toggle-btn">
            {{ isFormExpanded ? '折叠' : '展开' }}
          </n-button>
        </div>
        <transition name="expand">
          <div v-show="isFormExpanded" class="form-wrapper">
            <n-card class="form-container" size="small">
              <form @submit.prevent="handleCreate">
                <div class="form-grid">
                  <div class="form-group">
                    <label for="create-code">商品编号:</label>
                    <n-input id="create-code" v-model:value="createFormData.product_code" placeholder="A01" clearable required />
                  </div>
                  <div class="form-group">
                    <label for="create-name">商品名称:</label>
                    <n-input id="create-name" v-model:value="createFormData.name" placeholder="灵梦亚克力立牌" clearable required />
                  </div>
                  <div class="form-group">
                    <label for="create-price">默认价格 (¥):</label>
                    <n-input-number id="create-price" v-model:value="createFormData.default_price" :step="0.01" placeholder="45.00" required />
                  </div>
                  <div class="form-group">
                    <label for="create-category">商品分类:</label>
                  <n-input id="create-category" v-model:value="createFormData.category" placeholder="漫画、亚克力、毛绒玩具等" clearable />
                  </div>
                </div>
                
                <!-- 【核心修改】使用可复用的 ImageUploader 组件 -->
                <ImageUploader
                  label="预览图"
                  v-model="createFormFile"
                />

                <n-button type="primary" attr-type="submit" :disabled="isCreating">
                  {{ isCreating ? '添加中...' : '添加到仓库' }}
                </n-button>
                <p v-if="createError" class="error-message">{{ createError }}</p>
              </form>
            </n-card>
          </div>
        </transition>
      </div>
      
      <!-- 商品列表 -->
      <MasterProductList @edit="openEditModal" @toggleStatus="handleToggleStatus" />
    </main>

    <!-- 编辑模态框 -->
    <AppModal :show="isEditModalVisible" @close="closeEditModal">
      <template #header><h3>编辑商品</h3></template>
      <template #body>
        <form v-if="editableProduct" class="edit-form" @submit.prevent="handleUpdate">
          <div class="form-group">
            <label>商品编号:</label>
            <n-input v-model:value="editableProduct.product_code" clearable required />
          </div>
          <div class="form-group">
            <label>商品名称:</label>
            <n-input v-model:value="editableProduct.name" clearable required />
          </div>
          <div class="form-group">
            <label>默认价格 (¥):</label>
            <n-input-number v-model:value="editableProduct.default_price" :step="0.01" required />
          </div>
          <div class="form-group">
            <label>商品分类:</label>
            <n-input v-model:value="editableProduct.category" placeholder="漫画、亚克力、毛绒玩具等" clearable />
          </div>

          <!-- 【核心修改】再次使用 ImageUploader 组件 -->
          <ImageUploader
            label="更换预览图"
            :initial-image-url="editableProduct.image_url"
            v-model="editFormFile"
            @image-removed="handleEditImageRemoval"
          />

          <p v-if="editError" class="error-message">{{ editError }}</p>
        </form>
      </template>
      <template #footer>
        <n-space>
          <n-button @click="closeEditModal">取消</n-button>
          <n-button type="primary" @click="handleUpdate" :disabled="isUpdating">
            {{ isUpdating ? '保存中...' : '保存更改' }}
          </n-button>
        </n-space>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { useProductStore } from '@/stores/productStore';
import { useSyncStore } from '@/stores/syncStore';
import MasterProductList from '@/components/product/MasterProductList.vue';
import AppModal from '@/components/shared/AppModal.vue';
// 【新增】导入可复用的图片上传组件
import ImageUploader from '@/components/shared/ImageUploader.vue';
import { NCard, NInput, NInputNumber, NButton, NCheckbox, NSpace, useDialog, useMessage } from 'naive-ui';

const store = useProductStore();
const syncStore = useSyncStore();
const dialog = useDialog();
const message = useMessage();
// 【移除】不再需要硬编码的 backendUrl

// --- 创建逻辑的状态 ---
const isCreating = ref(false);

const createError = ref('');
const createFormData = ref({ product_code: '', name: '', default_price: null });
const createFormFile = ref(null); // ImageUploader 会通过 v-model 更新这个
// 【移除】不再需要 createPreviewUrl 和 handleCreateFileChange

async function handleCreate() {
  isCreating.value = true;
  createError.value = '';
  try {
    // 【修改】构建 FormData
    const formData = new FormData();
    const code = String(createFormData.value.product_code || '').trim();
    const name = String(createFormData.value.name || '').trim();
    const price = createFormData.value.default_price;
    const category = String(createFormData.value.category ?? '').trim();
    formData.append('product_code', code);
    formData.append('name', name);
    formData.append('default_price', String(price));
    if (category) formData.append('category', category);
    if (createFormFile.value) {
      formData.append('image', createFormFile.value);
    }
    
    // 假设 store action 已被修改为接收 FormData
    await store.createMasterProduct(formData);
    
    // 成功后重置表单
    createFormData.value = { product_code: '', name: '', default_price: null };
    createFormFile.value = null; // ImageUploader 会自动清空预览
  } catch (error) {
    createError.value = error.message;
  } finally {
    isCreating.value = false;
  }
}

// --- 编辑逻辑的状态 ---
const isEditModalVisible = ref(false);
const isUpdating = ref(false);
const editError = ref('');
const editableProduct = ref({});
const editFormFile = ref(null); // ImageUploader 会通过 v-model 更新这个
const isImageRemovedForEdit = ref(false); // 标记用户是否点击了移除
// --- 同步逻辑的状态 ---
const importFileInputRef = ref(null);
const syncMessage = ref('');
const syncError = ref('');
const isExporting = computed(() => syncStore.isExporting);
const isImporting = computed(() => syncStore.isImporting);
const isDragging = ref(false);
const isSyncExpanded = ref(true);
const isFormExpanded = ref(true);
let tauriUnlisten = null;
let globalDropCleanup = null;
// 【移除】不再需要 editPreviewUrl 和 handleEditFileChange

function openEditModal(product) {
  editableProduct.value = { ...product };
  editFormFile.value = null; // 清空上次选择的文件
  isImageRemovedForEdit.value = false; // 重置移除标记
  isEditModalVisible.value = true;
}

function closeEditModal() {
  isEditModalVisible.value = false;
  editError.value = '';
}

// 【新增】处理来自 ImageUploader 的移除事件
function handleEditImageRemoval() {
  isImageRemovedForEdit.value = true;
}

async function handleUpdate() {
  isUpdating.value = true;
  editError.value = '';
  try {
    // 【修改】构建 FormData
    const formData = new FormData();
    const eCode = String(editableProduct.value.product_code || '').trim();
    const eName = String(editableProduct.value.name || '').trim();
    const ePrice = editableProduct.value.default_price;
    const eCategory = String(editableProduct.value.category ?? '').trim();
    formData.append('product_code', eCode);
    formData.append('name', eName);
    formData.append('default_price', String(ePrice));
    if (eCategory) formData.append('category', eCategory);
    if (editFormFile.value) {
      formData.append('image', editFormFile.value);
    } else if (isImageRemovedForEdit.value) {
      formData.append('remove_image', 'true');
    }
    
    // 假设 store action 已被修改为接收 ID 和 FormData
    await store.updateMasterProduct(editableProduct.value.id, formData);
    closeEditModal();
  } catch (error) {
    editError.value = error.message;
  } finally {
    isUpdating.value = false;
  }
}

async function handleToggleStatus(product) {
  const actionText = product.is_active ? '停用' : '启用';
  if (window.confirm(`确定要"${actionText}"商品 "${product.name}" 吗？`)) {
    try {
      await store.toggleProductStatus(product);
    } catch (error) {
      alert(error.message);
    }
  }
}

// --- 导出 / 导入 ---
function triggerImport() {
  syncMessage.value = '';
  syncError.value = '';
  importFileInputRef.value?.click();
}

async function handleImportFile(event) {
  const file = event.target.files?.[0];
  if (!file) return;
  console.log('[Import] input change file:', file.name, file.size);
  await handleFileLike(file);
  event.target.value = '';
}

async function handleExport() {
  syncMessage.value = '';
  syncError.value = '';
  try {
    const { filename } = await syncStore.exportProducts();
    syncMessage.value = `已导出 ${filename}`;
    message.success(`已成功导出制品包：${filename}`, {
      duration: 5000,
      closable: true
    });
  } catch (error) {
    syncError.value = error.message;
    message.error(`导出失败：${error.message}`, {
      duration: 5000,
      closable: true
    });
  }
}

// --- 拖拽导入 + Tauri 文件拖入 ---
function onDragEnter(event) {
  event.stopPropagation();
  isDragging.value = true;
  console.log('[Drag] enter');
}

function onDragOver() {
  isDragging.value = true;
  // 不做重置消息，保持静默
}

function onDragLeave() {
  isDragging.value = false;
  console.log('[Drag] leave');
}

async function handleFileLike(file) {
  if (!file) return;
  const name = file.name || 'unknown';
  console.log('[Import] handleFileLike:', name, file.size);
  const lowered = name.toLowerCase();
  if (!(lowered.endsWith('.boothpack') || lowered.endsWith('.zip'))) {
    syncError.value = '请拖入 .boothpack 或 .zip 文件';
    console.warn('[Import] invalid extension', name);
    return;
  }
  
  // 使用 Naive UI 弹出确认框
  dialog.warning({
    title: '确认导入',
    content: `确定要导入文件 "${name}" 吗？\n这将覆盖或更新现有的商品数据。`,
    positiveText: '确定导入',
    negativeText: '取消',
    onPositiveClick: async () => {
      syncMessage.value = '';
      syncError.value = '';
      try {
        const result = await syncStore.importProducts(file);
        await store.fetchMasterProducts();
        const count = result?.count ?? '若干';
        syncMessage.value = `导入成功，更新 ${count} 条制品。`;
        message.success(`导入成功！已更新 ${count} 条制品数据`, {
          duration: 5000,
          closable: true
        });
      } catch (error) {
        syncError.value = error.message;
        message.error(`导入失败：${error.message}`, {
          duration: 5000,
          closable: true
        });
      }
    }
  });
}

async function onDrop(event) {
  isDragging.value = false;
  console.log('[Drag] drop payload files:', event.dataTransfer?.files?.length);
  const file = event.dataTransfer?.files?.[0];
  if (file) {
    await handleFileLike(file);
  }
  event.dataTransfer?.clearData?.();
}

// Tauri v2 专用文件拖入（路径）
// 使用 listen API 监听后端 emit 的 'boothpack-file-drop' 事件
async function setupTauriFileDrop() {
  if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
    console.log('[Tauri] not in Tauri environment, skipping file-drop setup');
    return;
  }

  try {
    // Tauri v2 API: 导入 listen 函数
    const { listen } = await import('@tauri-apps/api/event');
    console.log('[Tauri] setting up boothpack-file-drop listener');

    const unlistenFns = [];

    // 处理路径的通用函数
    const handlePaths = async (paths) => {
      const first = Array.isArray(paths) ? paths[0] : paths;
      if (!first || typeof first !== 'string') {
        console.warn('[Tauri] invalid paths:', paths);
        return;
      }

      // 提取文件名用于确认框显示
      const fileName = first.split(/[/\\]/).pop() || 'import.boothpack';
      
      // 使用 Naive UI 弹出确认框
      dialog.warning({
        title: '检测到文件拖入',
        content: `文件名：${fileName}\n\n确定要导入吗？这将覆盖或更新现有的商品数据。`,
        positiveText: '确定导入',
        negativeText: '取消',
        onPositiveClick: async () => {
          syncMessage.value = '';
          syncError.value = '';
          try {
            console.log('[Tauri] importing from path:', first);
            const result = await syncStore.importProductsFromPath(first, true); // skipConfirm = true
            await store.fetchMasterProducts();
            const count = result?.count ?? '若干';
            syncMessage.value = `导入成功，更新 ${count} 条制品。`;
            message.success(`导入成功！已更新 ${count} 条制品数据`, {
              duration: 5000,
              closable: true
            });
          } catch (err) {
            syncError.value = err.message || '导入失败';
            message.error(`导入失败：${err.message || '未知错误'}`, {
              duration: 5000,
              closable: true
            });
            console.error('[Tauri] import from path failed:', err);
          }
        }
      });
    };

    // 【主要】监听后端通过 emit("boothpack-file-drop", paths) 发送的事件
    // 在 lib.rs 中，backend 会在检测到文件拖入时调用:
    // let _ = main_clone.emit("boothpack-file-drop", paths.clone());
    try {
      const unlisten = await listen('boothpack-file-drop', (event) => {
        console.log('[Tauri] received boothpack-file-drop event:', event.payload);
        // event.payload 直接是后端 emit 的 paths 数组
        handlePaths(event.payload);
      });
      unlistenFns.push(unlisten);
      console.log('[Tauri] boothpack-file-drop listener registered');
    } catch (err) {
      console.warn('[Tauri] failed to listen boothpack-file-drop:', err);
    }

    // 【备选】也可监听官方的 tauri://file-drop 事件（如果权限允许）
    try {
      const unlisten = await listen('tauri://file-drop', (event) => {
        console.log('[Tauri] received tauri://file-drop event:', event.payload);
        const paths = event.payload?.paths || event.payload;
        handlePaths(paths);
      });
      unlistenFns.push(unlisten);
      console.log('[Tauri] tauri://file-drop listener registered (fallback)');
    } catch (err) {
      console.warn('[Tauri] tauri://file-drop not available (expected):', err.message);
    }

    // 存储所有 unlisten 函数，以便组件卸载时清理
    tauriUnlisten = () => {
      console.log('[Tauri] cleaning up', unlistenFns.length, 'event listeners');
      unlistenFns.forEach((fn) => {
        if (typeof fn === 'function') {
          fn();
        }
      });
      unlistenFns.length = 0;
    };
  } catch (err) {
    console.error('[Tauri] failed to setup file-drop listeners:', err);
  }
}

onMounted(async () => {
  // 全局阻止默认的拖拽打开行为，确保 drop 事件留在应用内
  const handleWindowDragOver = (e) => {
    e.preventDefault();
  };
  const handleWindowDrop = (e) => {
    e.preventDefault();
    console.log('[Drag] window drop (global) blocked');
  };
  window.addEventListener('dragover', handleWindowDragOver);
  window.addEventListener('drop', handleWindowDrop);
  globalDropCleanup = () => {
    window.removeEventListener('dragover', handleWindowDragOver);
    window.removeEventListener('drop', handleWindowDrop);
  };

  // 设置 Tauri 文件拖放监听
  await setupTauriFileDrop();
});

onBeforeUnmount(() => {
  // 清理 Tauri 事件监听器
  if (typeof tauriUnlisten === 'function') {
    console.log('[Tauri] unregistering event listeners');
    tauriUnlisten();
  }
  // 清理全局拖拽事件处理
  if (typeof globalDropCleanup === 'function') {
    console.log('[Drag] cleaning up global listeners');
    globalDropCleanup();
  }
});
</script>

<style scoped>
/* 页面头部 */
.page-header {
  margin-bottom: 1.5rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--border-color);
}

.header-content h1 { 
  color: var(--accent-color); 
  margin: 0;
  font-size: 1.5rem;
}

.header-content p { 
  color: var(--text-muted); 
  margin-top: 0.5rem;
  font-size: 0.95rem;
}

/* 通用区块样式 */
.sync-section,
.form-section {
  margin-bottom: 2rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: 8px;
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}

.section-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.section-header h2 {
  margin: 0;
  font-size: 1.25rem;
  color: var(--accent-color);
  font-weight: 600;
}

.toggle-btn {
  font-size: 0.9rem;
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

/* 展开/折叠动画 */
.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  opacity: 1;
  max-height: 2000px;
}

/* 制品包管理区域 */
.sync-container,
.form-wrapper {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: 8px;
  padding: 1.5rem;
}

.info-box {
  background: var(--bg-secondary);
  border-left: 4px solid var(--accent-color);
  padding: 1rem 1.25rem;
  border-radius: 6px;
  margin-bottom: 1.5rem;
}

.info-text {
  margin: 0 0 0.75rem 0;
  line-height: 1.6;
  color: var(--primary-text-color);
  font-size: 0.95rem;
}

.info-text:last-child {
  margin-bottom: 0;
}

.info-text strong {
  color: var(--accent-color);
  font-weight: 600;
}

.info-text code {
  background: var(--input-bg-color);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.9em;
  color: var(--accent-color);
}

.info-warning strong {
  color: var(--warning-color);
}

.section-content {
  margin-top: 1rem;
}

.sync-controls {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.btn-icon {
  font-size: 18px;
}

.hidden-input {
  display: none;
}

/* 拖拽区域 */
.drop-zone {
  margin-top: 1rem;
  padding: 2rem;
  border: 2px dashed var(--border-color);
  border-radius: 10px;
  background: var(--card-bg-color);
  transition: all 0.3s ease;
  cursor: pointer;
}

.drop-zone:hover {
  border-color: var(--accent-color);
  background: var(--hover-bg-color, var(--card-bg-color));
}

.drop-zone.is-dragging {
  border-color: var(--accent-color);
  border-width: 3px;
  background: var(--active-bg-color, var(--card-bg-color));
  transform: scale(1.02);
  box-shadow: 0 4px 12px var(--shadow-color, rgba(0, 0, 0, 0.1));
}

.drop-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.drop-zone-icon {
  font-size: 2.5rem;
  line-height: 1;
}

.drop-zone-text {
  color: var(--text-muted);
  font-size: 1rem;
  text-align: center;
}

.drop-zone.is-dragging .drop-zone-text {
  color: var(--accent-color);
  font-weight: 600;
}

/* 表单容器 */
.form-wrapper {
  margin-top: 0;
}

.form-container {
  background-color: transparent;
  border: none;
  padding: 0;
  border-radius: 0;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
  margin-bottom: 1.5rem;
}

.form-group { 
  display: flex; 
  flex-direction: column; 
}

label { 
  margin-bottom: 0.5rem;
  font-weight: 500;
}

.error-message { 
  color: var(--error-color); 
  margin-top: 1rem; 
}

.btn-primary {
  background-color: var(--accent-color);
  color: var(--bg-color);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
</style>