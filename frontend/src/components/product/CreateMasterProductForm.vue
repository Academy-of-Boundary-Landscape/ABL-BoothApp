<template>
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
            <div class="form-layout">
              <div class="form-fields">
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
                    <label for="create-price">默认价格（元）:</label>
                    <n-input-number
                      id="create-price"
                      v-model:value="createFormData.default_price"
                      :step="0.01"
                      :show-button="false"
                      placeholder="45.00"
                      required
                    />
                  </div>

                  <div class="form-group">
                    <label for="create-category">商品分类:</label>
                    <n-select
                      id="create-category"
                      v-model:value="createFormData.category"
                      :options="store.categoryOptions"
                      filterable
                      tag
                      clearable
                      placeholder="可选择已有分类，或直接输入新分类"
                    />
                  </div>

                  <div class="form-group" style="grid-column: 1 / -1;">
                    <label for="create-tags">标签:</label>
                    <n-select
                      id="create-tags"
                      v-model:value="createFormData.tags"
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
                  label="商品预览图"
                  v-model="createFormFile"
                  @invalid-file="handleInvalidFile"
                />
              </div>
            </div>

            <n-button type="primary" attr-type="submit" :disabled="isCreating">
              {{ isCreating ? '添加中...' : '添加到仓库' }}
            </n-button>

            <p v-if="createError" class="error-message">{{ createError }}</p>
          </form>
        </n-card>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { NButton, NCard, NInput, NInputNumber, NSelect } from 'naive-ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'
import { useProductStore } from '@/stores/productStore'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  normalizeUploadError,
} from '@/utils/upload'

const emit = defineEmits(['created'])
const store = useProductStore()

const isCreating = ref(false)
const createError = ref('')
const isFormExpanded = ref(true)

const createFormData = ref({
  product_code: '',
  name: '',
  default_price: null,
  category: '',
  tags: [],
})

const createFormFile = ref(null)

function handleInvalidFile(message) {
  createError.value = message
}

async function handleCreate() {
  isCreating.value = true
  createError.value = ''

  try {
    const formData = new FormData()
    const code = String(createFormData.value.product_code || '').trim()
    const name = String(createFormData.value.name || '').trim()
    const price = createFormData.value.default_price
    const category = String(createFormData.value.category ?? '').trim()

    if (!code || !name || price == null) {
      throw new Error('请填写商品编号、名称和默认价格')
    }

    formData.append('product_code', code)
    formData.append('name', name)
    formData.append('default_price', String(price))
    if (category) formData.append('category', category)

    formData.append('tags', (createFormData.value.tags || []).join(','))

    if (createFormFile.value) {
      formData.append('image', createFormFile.value)
    }

    await store.createMasterProduct(formData)

    createFormData.value = {
      product_code: '',
      name: '',
      default_price: null,
      category: '',
      tags: [],
    }
    createFormFile.value = null
    emit('created')
  } catch (error) {
    createError.value = normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB)
  } finally {
    isCreating.value = false
  }
}
</script>

<style scoped>
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
  border-radius: var(--radius-md);
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}

.section-header:hover {
  background: var(--hover-bg-color, var(--card-bg-color));
  border-color: var(--accent-color);
}

.section-header h2 {
  margin: 0;
  font-size: var(--font-lg);
  color: var(--accent-color);
  font-weight: 600;
}

.toggle-btn {
  font-size: var(--font-base);
  padding: 0.25rem 0.75rem;
  min-width: auto;
  color: var(--accent-color);
}

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

.form-wrapper {
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.5rem;
}

.form-container {
  background-color: transparent;
  border: none;
  padding: 0;
  border-radius: 0;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}

.form-layout {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(260px, 320px);
  gap: 1.25rem;
  align-items: start;
  margin-bottom: 1rem;
}

.form-fields,
.form-media {
  min-width: 0;
}

.form-group {
  display: flex;
  flex-direction: column;
}

label {
  margin-bottom: 0.35rem;
  font-weight: 500;
}

.error-message {
  color: var(--error-color);
  margin-top: 0.75rem;
}

.form-group :deep(.n-input-number) {
  width: 100%;
}

@media (max-width: 900px) {
  .form-layout {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
