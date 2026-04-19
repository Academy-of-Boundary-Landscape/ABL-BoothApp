<template>
  <section class="form-container">
    <div class="section-header" @click="isCollapsed = !isCollapsed">
      <h2>创建新展会</h2>
      <n-button text class="toggle-btn">
        {{ isCollapsed ? '展开' : '折叠' }}
      </n-button>
    </div>

    <transition name="expand">
      <div v-show="!isCollapsed" class="section-container">
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
            <p v-if="errorMessage" class="error-message">{{ errorMessage }}</p>
          </div>
        </n-form>
      </div>
    </transition>
  </section>
</template>

<script setup>
import { ref } from 'vue'
import { NButton, NDatePicker, NForm, NInput } from 'naive-ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'
import { useEventStore } from '@/stores/eventStore'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  normalizeUploadError,
  showUploadDialog,
} from '@/utils/upload'

const store = useEventStore()
const isSubmitting = ref(false)
const errorMessage = ref('')
const isCollapsed = ref(false)

const formData = ref({
  name: '',
  date: null,
  location: '',
  vendor_password: '',
})

const qrCodeWechat = ref(null)
const qrCodeAlipay = ref(null)

function handleInvalidFile(message) {
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
    qrCodeWechat.value = null
    qrCodeAlipay.value = null
  } catch (error) {
    errorMessage.value = normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB)
  } finally {
    isSubmitting.value = false
  }
}
</script>

<style scoped>
.form-container {
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
  max-height: 0;
  opacity: 0;
}

.expand-enter-to,
.expand-leave-from {
  max-height: 2000px;
  opacity: 1;
}

.section-container {
  background-color: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.5rem;
}

.two-column-form {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.6rem 1rem;
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
  margin-bottom: 0.3rem;
  font-size: 0.95em;
  font-weight: 500;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.error-message {
  color: var(--error-color);
  margin: 0;
}
</style>
