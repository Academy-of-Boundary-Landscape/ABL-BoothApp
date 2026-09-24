<template>
  <SectionCard
    class="form-container"
    title="创建新展会"
    collapsible
    v-model:collapsed="isCollapsed"
  >
    <n-form class="two-column-form" @submit.prevent>
      <div class="form-group">
        <label for="name">展会名称:</label>
        <n-input id="name" v-model:value="formData.name" placeholder="例如：COMICUP 31" />
      </div>

      <div class="form-group">
        <label for="date">日期:</label>
        <n-date-picker
          id="date"
          v-model:formatted-value="formData.date"
          type="date"
          value-format="yyyy-MM-dd"
        />
      </div>

      <div class="form-group">
        <label for="location">地点:</label>
        <n-input id="location" v-model:value="formData.location" placeholder="例如：上海" />
      </div>

      <div class="form-group">
        <label for="vendor_password">摊主密码（可选）:</label>
        <n-input
          id="vendor_password"
          v-model:value="formData.vendor_password"
          placeholder="留空则使用全局密码"
        />
      </div>

      <div class="form-group">
        <ImageUploader
          label="微信收款码（可选）"
          v-model="qrCodeWechat"
          @invalid-file="handleInvalidFile"
        />
      </div>

      <div class="form-group">
        <ImageUploader
          label="支付宝收款码（可选）"
          v-model="qrCodeAlipay"
          @invalid-file="handleInvalidFile"
        />
      </div>

      <div class="form-actions full-width">
        <n-button type="primary" :loading="isSubmitting" @click="handleSubmit">
          {{ isSubmitting ? '创建中...' : '创建' }}
        </n-button>
        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
      </div>
    </n-form>
  </SectionCard>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NDatePicker, NForm, NInput } from 'naive-ui'
import { SectionCard } from '@/components/ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'
import { useEventStore } from '@/stores/eventStore'
import { IMAGE_UPLOAD_LIMIT_MB, normalizeUploadError, showUploadDialog } from '@/utils/upload'

type EventFormData = {
  name: string
  date: string | null
  location: string
  vendor_password: string
}

const store = useEventStore()
const isSubmitting = ref(false)
const errorMessage = ref('')
const isCollapsed = ref(false)

const formData = ref<EventFormData>({
  name: '',
  date: null,
  location: '',
  vendor_password: '',
})

const qrCodeWechat = ref<File | undefined>(undefined)
const qrCodeAlipay = ref<File | undefined>(undefined)

function handleInvalidFile(message: string) {
  errorMessage.value = message
}

async function handleSubmit() {
  if (!formData.value.name || !formData.value.date) {
    errorMessage.value = '展会名称和日期不能为空。'
    showUploadDialog('表单未填写完整', errorMessage.value)
    return
  }

  isSubmitting.value = true
  errorMessage.value = ''

  const submissionData = new FormData()
  submissionData.append('name', formData.value.name)
  submissionData.append('date', formData.value.date)
  submissionData.append('location', formData.value.location)
  submissionData.append('vendor_password', formData.value.vendor_password)

  if (qrCodeWechat.value) {
    submissionData.append('payment_qr_code_wechat', qrCodeWechat.value)
  }
  if (qrCodeAlipay.value) {
    submissionData.append('payment_qr_code_alipay', qrCodeAlipay.value)
  }

  try {
    await store.createEvent(submissionData)
    formData.value = { name: '', date: null, location: '', vendor_password: '' }
    qrCodeWechat.value = undefined
    qrCodeAlipay.value = undefined
  } catch (error) {
    errorMessage.value = normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB)
  } finally {
    isSubmitting.value = false
  }
}
</script>

<style scoped>
.form-container {
  margin-bottom: var(--space-2xl);
}

.two-column-form {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-sm) var(--space-lg);
}

.full-width {
  grid-column: 1 / -1;
}

.form-group {
  margin-bottom: 0;
  display: flex;
  flex-direction: column;
}

label {
  display: block;
  margin-bottom: var(--space-xs);
  font-size: var(--font-base);
  font-weight: var(--weight-medium);
}

.form-actions {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.form-error {
  color: var(--error-color);
  margin: 0;
}
</style>
