<template>
  <n-modal
    :show="show"
    :mask-closable="true"
    @update:show="val => { if (!val) handleClose() }"
  >
    <n-card :bordered="true" size="medium" class="edit-modal-card">
      <template #header>
        <div class="modal-header">
          <h3 class="modal-title">编辑商品</h3>
          <n-button quaternary circle size="small" @click="handleClose">×</n-button>
        </div>
      </template>

      <div v-if="localProduct" class="modal-body">
        <n-tabs v-model:value="activeTab" type="line" animated>
          <!-- ==================== Tab 1: 基本信息 ==================== -->
          <n-tab-pane name="info" tab="基本信息">
            <form class="edit-form" @submit.prevent="handleUpdate">
              <div class="form-layout">
                <div class="form-fields">
                  <div class="form-grid">
                    <div class="form-group">
                      <label>商品编号:</label>
                      <n-input v-model:value="localProduct.product_code" clearable required />
                    </div>
                    <div class="form-group">
                      <label>商品名称:</label>
                      <n-input v-model:value="localProduct.name" clearable required />
                    </div>
                    <div class="form-group">
                      <label>默认价格（元）:</label>
                      <n-input-number
                        v-model:value="localProduct.default_price"
                        :step="0.01"
                        :precision="2"
                        :show-button="false"
                        required
                        style="width: 100%;"
                      />
                    </div>
                    <div class="form-group">
                      <label>商品分类:</label>
                      <n-select
                        v-model:value="localProduct.category"
                        :options="store.categoryOptions"
                        filterable
                        tag
                        clearable
                        placeholder="可选择已有分类，或直接输入新分类"
                      />
                    </div>

                    <div class="form-group" style="grid-column: 1 / -1;">
                      <label>标签:</label>
                      <n-select
                        v-model:value="localProduct.tags"
                        :options="store.tagOptions"
                        placeholder="选择或输入标签（如角色名、系列）"
                        filterable
                        tag
                        multiple
                        clearable
                      />
                    </div>
                  </div>
                </div>

                <div class="form-media">
                  <ImageUploader
                    label="更换商品预览图"
                    :initial-image-url="localProduct.image_url"
                    v-model="editFormFile"
                    @image-removed="handleImageRemoval"
                    @invalid-file="handleInvalidFile"
                  />
                </div>
              </div>
              <p v-if="editError" class="error-message">{{ editError }}</p>
            </form>
          </n-tab-pane>

          <!-- ==================== Tab 2: 识别用图片 ==================== -->
          <n-tab-pane name="gallery" tab="识别用图片">
            <div class="gallery-section">
              <n-alert :bordered="false" type="info" class="gallery-hint">
                上传商品不同角度的照片，系统会用这些图片学习识别该商品。照片越多、角度越丰富，拍照识别越准确。
                <br /><strong>建议：</strong>使用接近正方形的照片（商品居中、背景简洁），上传后会自动压缩到 512×512。
              </n-alert>

              <!-- 加载中 -->
              <div v-if="galleryLoading" class="gallery-loading">
                <n-spin size="small" /> 加载中...
              </div>

              <template v-else>
                <!-- 图片网格 -->
                <div class="gallery-grid">
                  <div
                    v-for="img in galleryImages"
                    :key="img.id"
                    class="gallery-item"
                  >
                    <div class="gallery-thumb">
                      <n-image
                        :src="resolveUrl(img.image_url)"
                        :alt="img.kind"
                        object-fit="cover"
                        class="gallery-img"
                      />
                    </div>
                    <div class="gallery-item-footer">
                      <div class="footer-tags">
                        <n-tag size="tiny" :type="kindTagType(img.kind)">
                          {{ kindLabel(img.kind) }}
                        </n-tag>
                        <n-tag
                          size="tiny"
                          :type="img.has_embedding ? 'success' : 'warning'"
                          :bordered="false"
                        >
                          {{ img.has_embedding ? '已嵌入' : '未嵌入' }}
                        </n-tag>
                      </div>
                      <n-button
                        size="tiny"
                        type="error"
                        tertiary
                        :disabled="galleryDeleting === img.id"
                        :loading="galleryDeleting === img.id"
                        @click="handleDeleteImage(img)"
                      >
                        删除
                      </n-button>
                    </div>
                  </div>

                  <!-- 添加按钮 -->
                  <div
                    class="gallery-item gallery-add"
                    :class="{ 'gallery-add--uploading': galleryUploading }"
                    @click="!galleryUploading && triggerGalleryUpload()"
                  >
                    <div class="gallery-add-inner">
                      <n-spin v-if="galleryUploading" size="small" />
                      <template v-else>
                        <span class="gallery-add-icon">+</span>
                        <span class="gallery-add-text">添加图片</span>
                      </template>
                    </div>
                  </div>
                </div>

                <!-- 空状态（仅在没有任何图片时） -->
                <div v-if="galleryImages.length === 0" class="gallery-empty">
                  还没有识别用图片。点击上方 "+" 添加商品照片，或在「基本信息」中上传预览图后会自动同步过来。
                </div>

                <!-- 图片数量提示 -->
                <div v-else class="gallery-count">
                  已有 {{ galleryImages.length }} 张识别用图片
                </div>
              </template>

              <input
                ref="galleryFileRef"
                type="file"
                accept="image/*"
                multiple
                class="hidden-input"
                @change="handleGalleryFileSelected"
              />

              <p v-if="galleryError" class="error-message">{{ galleryError }}</p>
            </div>
          </n-tab-pane>
        </n-tabs>
      </div>

      <div v-else class="empty-hint">未选择要编辑的商品</div>

      <template #footer>
        <div class="modal-footer">
          <n-space justify="end">
            <n-button @click="handleClose">取消</n-button>
            <n-button
              v-if="activeTab === 'info'"
              type="primary"
              :loading="isUpdating"
              :disabled="isUpdating || !localProduct"
              @click="handleUpdate"
            >
              {{ isUpdating ? '保存中...' : '保存更改' }}
            </n-button>
          </n-space>
        </div>
      </template>
    </n-card>
  </n-modal>
</template>

<script setup>
import { ref, watch } from 'vue'
import {
  NModal, NCard, NButton, NInput, NInputNumber, NSelect,
  NSpace, NTabs, NTabPane, NAlert, NTag, NImage, NSpin,
} from 'naive-ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'
import { useProductStore } from '@/stores/productStore'
import { getImageUrl } from '@/services/url'
import {
  listProductImages,
  addProductImage,
  deleteProductImage,
} from '@/services/vision'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  normalizeUploadError,
  validateFileSize,
  showUploadDialog,
  resizeImageFile,
} from '@/utils/upload'

const GALLERY_RESIZE_PX = 512

const props = defineProps({
  show: { type: Boolean, default: false },
  product: { type: Object, default: null },
})

const emit = defineEmits(['close', 'updated'])
const store = useProductStore()

const activeTab = ref('info')
const isUpdating = ref(false)
const editError = ref('')
const localProduct = ref(null)
const editFormFile = ref(null)
const isImageRemovedForEdit = ref(false)

// ===== 基本信息 Tab =====
watch(
  () => props.product,
  (product) => {
    if (!product) {
      localProduct.value = null
      editFormFile.value = null
      isImageRemovedForEdit.value = false
      editError.value = ''
      return
    }
    localProduct.value = {
      ...product,
      tags: (product.tags || '').split(',').filter(Boolean),
    }
    editFormFile.value = null
    isImageRemovedForEdit.value = false
    editError.value = ''
    // product 变化时立即加载识别用图片
    if (props.show) loadGallery()
  },
  { immediate: true }
)

watch(
  () => props.show,
  (visible) => {
    if (!visible) {
      editError.value = ''
      editFormFile.value = null
      isImageRemovedForEdit.value = false
      activeTab.value = 'info'
      galleryImages.value = []
      galleryError.value = ''
    } else if (visible && localProduct.value) {
      // 弹窗打开时也加载一次（覆盖 product watch 可能的时序问题）
      loadGallery()
    }
  }
)

function handleInvalidFile(message) { editError.value = message }
function handleImageRemoval() { isImageRemovedForEdit.value = true }
function handleClose() { emit('close') }

async function handleUpdate() {
  if (!localProduct.value || isUpdating.value) return
  isUpdating.value = true
  editError.value = ''

  try {
    const formData = new FormData()
    const code = String(localProduct.value.product_code || '').trim()
    const name = String(localProduct.value.name || '').trim()
    const price = localProduct.value.default_price
    const category = String(localProduct.value.category ?? '').trim()

    if (!code || !name || price == null) {
      throw new Error('请填写商品编号、名称和默认价格')
    }

    formData.append('product_code', code)
    formData.append('name', name)
    formData.append('default_price', String(price))
    if (category) formData.append('category', category)

    formData.append('tags', (localProduct.value.tags || []).join(','))

    if (editFormFile.value) {
      formData.append('image', editFormFile.value)
    } else if (isImageRemovedForEdit.value) {
      formData.append('remove_image', 'true')
    }

    await store.updateMasterProduct(localProduct.value.id, formData)
    emit('updated')
    emit('close')
  } catch (error) {
    editError.value = normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB)
  } finally {
    isUpdating.value = false
  }
}

// ===== 识别用图片 Tab =====
const galleryImages = ref([])
const galleryLoading = ref(false)
const galleryUploading = ref(false)
const galleryError = ref('')
const galleryDeleting = ref(null)
const galleryFileRef = ref(null)

function resolveUrl(url) { return getImageUrl(url) }

function kindLabel(kind) {
  const map = {
    legacy_main: '主图',
    gallery: '识别图',
    feedback: '反馈',
    feedback_incorrect: '负样本',
  }
  return map[kind] || kind
}

function kindTagType(kind) {
  if (kind === 'legacy_main') return 'default'
  if (kind === 'feedback_incorrect') return 'error'
  return 'info'
}

async function loadGallery() {
  if (!localProduct.value) return
  galleryLoading.value = true
  galleryError.value = ''
  try {
    galleryImages.value = await listProductImages(localProduct.value.id)
  } catch {
    galleryError.value = '加载图片列表失败'
    galleryImages.value = []
  } finally {
    galleryLoading.value = false
  }
}

function triggerGalleryUpload() {
  galleryFileRef.value?.click()
}

async function handleGalleryFileSelected(e) {
  const files = Array.from(e.target.files || [])
  e.target.value = ''
  if (!files.length || !localProduct.value) return

  galleryError.value = ''
  galleryUploading.value = true

  try {
    for (const file of files) {
      const validation = validateFileSize(file, IMAGE_UPLOAD_LIMIT_MB)
      if (!validation.ok) {
        showUploadDialog('图片过大', `${file.name}: ${validation.message}`)
        continue
      }
      // AI 识别图强制缩放到 512px，减少存储和推理预处理开销
      const resized = await resizeImageFile(file, GALLERY_RESIZE_PX)
      await addProductImage(localProduct.value.id, resized, 'gallery')
    }
    await loadGallery()
  } catch (err) {
    galleryError.value = normalizeUploadError(err, IMAGE_UPLOAD_LIMIT_MB)
  } finally {
    galleryUploading.value = false
  }
}

async function handleDeleteImage(img) {
  if (!localProduct.value) return

  // 如果是主图同步过来的，给个提示
  if (img.kind === 'legacy_main') {
    const ok = confirm('这是从商品预览图自动同步的图片。删除后如需恢复，请在「基本信息」中重新上传预览图。确认删除？')
    if (!ok) return
  }

  galleryDeleting.value = img.id
  galleryError.value = ''
  try {
    await deleteProductImage(localProduct.value.id, img.id)
    galleryImages.value = galleryImages.value.filter((i) => i.id !== img.id)
  } catch (err) {
    galleryError.value = err.response?.data?.error || '删除失败'
  } finally {
    galleryDeleting.value = null
  }
}
</script>

<style scoped>
.edit-modal-card {
  width: 680px;
  max-width: 92vw;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.modal-title { margin: 0; }

.modal-body {
  padding: 0.5rem 0;
}

.modal-footer {
  border-top: 1px solid var(--border-color);
  padding-top: 0.75rem;
}

.hidden-input { display: none; }

/* ===== 基本信息 Tab ===== */
.edit-form {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}
.form-layout {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
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
  margin-top: 0.25rem;
  font-size: var(--font-base);
}
.empty-hint {
  color: var(--text-muted);
  padding: 1rem 0;
}

/* ===== 识别用图片 Tab ===== */
.gallery-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.gallery-hint {
  font-size: var(--font-sm);
}
.gallery-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  padding: 16px 0;
  font-size: var(--font-sm);
}

.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 10px;
}

.gallery-item {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-color);
  display: flex;
  flex-direction: column;
}

.gallery-thumb {
  aspect-ratio: 1;
  overflow: hidden;
  background: var(--bg-secondary);
}

.gallery-img {
  width: 100%;
  height: 100%;
}
.gallery-img :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.gallery-item-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 6px;
  gap: 4px;
}
.footer-tags {
  display: flex;
  gap: 3px;
  flex-wrap: wrap;
  min-width: 0;
}

/* 添加按钮卡片 */
.gallery-add {
  cursor: pointer;
  border: 2px dashed var(--border-color);
  background: transparent;
  transition: border-color 0.15s, background-color 0.15s;
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.gallery-add:hover {
  border-color: var(--accent-color);
  background: var(--hover-bg-color);
}
.gallery-add--uploading {
  pointer-events: none;
  opacity: 0.7;
}
.gallery-add-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  color: var(--text-muted);
}
.gallery-add-icon {
  font-size: 1.8rem;
  line-height: 1;
}
.gallery-add-text {
  font-size: var(--font-sm);
}

.gallery-empty {
  color: var(--text-muted);
  font-size: var(--font-sm);
  padding: 8px 0;
  text-align: center;
}

.gallery-count {
  font-size: var(--font-sm);
  color: var(--text-muted);
  text-align: right;
}

@media (max-width: 640px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
  .gallery-grid {
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  }
}
</style>
