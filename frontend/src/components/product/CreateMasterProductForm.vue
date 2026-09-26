<template>
  <SectionCard
    class="form-section"
    title="添加新商品到仓库"
    collapsible
    v-model:collapsed="isFormCollapsed"
  >
    <form @submit.prevent="handleCreate">
      <div class="form-layout">
        <div class="form-fields">
          <div class="form-grid">
            <div class="form-group">
              <label for="create-code">商品编号:</label>
              <n-input
                id="create-code"
                v-model:value="createFormData.product_code"
                placeholder="A01"
                clearable
                required
              />
            </div>

            <div class="form-group">
              <label for="create-barcode">商业条码（可选）:</label>
              <n-input
                id="create-barcode"
                v-model:value="createFormData.barcode"
                placeholder="JAN / ISBN 等，没有就留空"
                clearable
              />
            </div>

            <div class="form-group">
              <label for="create-name">商品名称:</label>
              <n-input
                id="create-name"
                v-model:value="createFormData.name"
                placeholder="灵梦亚克力立牌"
                clearable
                required
              />
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

            <div class="form-group">
              <label>所属社团:</label>
              <SocietySelect
                v-model="createFormData.owner_society_id"
                placeholder="不选则归本社团"
              />
            </div>

            <div class="form-group" style="grid-column: 1 / -1">
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
            crop-enabled
            :crop-default-aspect="themeStore.productImageAspect"
            @invalid-file="handleInvalidFile"
          />
        </div>
      </div>

      <n-button type="primary" attr-type="submit" :disabled="isCreating">
        {{ isCreating ? '添加中...' : '添加到仓库' }}
      </n-button>

      <p v-if="createError" class="form-error">{{ createError }}</p>
    </form>
  </SectionCard>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NInput, NInputNumber, NSelect } from 'naive-ui'
import { SectionCard } from '@/components/ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'
import SocietySelect from '@/components/shared/SocietySelect.vue'
import { useProductStore } from '@/stores/productStore'
import { useThemeStore } from '@/stores/themeStore'
import { IMAGE_UPLOAD_LIMIT_MB, normalizeUploadError } from '@/utils/upload'

/** 表单内部状态：价格是元（数字）、标签是数组，与 multipart 契约的字符串字段不同。 */
interface CreateFormState {
  product_code: string
  /** 商业条码；空串提交表示不填。 */
  barcode: string
  name: string
  default_price: number | null
  category: string
  tags: string[]
  /** 所属社团（货主）；null = 不传，后端归本社团。 */
  owner_society_id: number | null
}

const emit = defineEmits<{ (e: 'created'): void }>()
const store = useProductStore()
const themeStore = useThemeStore()

const isCreating = ref(false)
const createError = ref('')
const isFormCollapsed = ref(false)

const createFormData = ref<CreateFormState>({
  product_code: '',
  barcode: '',
  name: '',
  default_price: null,
  category: '',
  tags: [],
  owner_society_id: null,
})

const createFormFile = ref<File | undefined>(undefined)

function handleInvalidFile(message: string) {
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
    // 总是提交（空串表示不填）；后端规范化后为空会存 NULL。
    formData.append('barcode', String(createFormData.value.barcode ?? '').trim())
    formData.append('name', name)
    formData.append('default_price', String(price))
    if (category) formData.append('category', category)

    formData.append('tags', (createFormData.value.tags || []).join(','))
    if (createFormData.value.owner_society_id !== null) {
      formData.append('owner_society_id', String(createFormData.value.owner_society_id))
    }

    if (createFormFile.value) {
      formData.append('image', createFormFile.value)
    }

    await store.createMasterProduct(formData)

    createFormData.value = {
      product_code: '',
      barcode: '',
      name: '',
      default_price: null,
      category: '',
      tags: [],
      owner_society_id: null,
    }
    createFormFile.value = undefined
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
  margin-bottom: var(--space-2xl);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-lg);
}

.form-layout {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(260px, 320px);
  gap: var(--space-lg);
  align-items: start;
  margin-bottom: var(--space-lg);
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
  margin-bottom: var(--space-sm);
  font-weight: var(--weight-medium);
}

.form-error {
  color: var(--error-color);
  margin-top: var(--space-md);
}

.form-group :deep(.n-input-number) {
  width: 100%;
}

@media (--tablet) {
  .form-layout {
    grid-template-columns: 1fr;
  }
}

@media (--phone) {
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
