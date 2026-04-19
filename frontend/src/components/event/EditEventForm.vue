<template>
  <n-form @submit.prevent>
    <div class="form-group">
      <label for="edit-name">展会名称:</label>
      <n-input id="edit-name" v-model:value="editableEvent.name" />
    </div>

    <div class="form-group">
      <label for="edit-date">日期:</label>
      <n-date-picker
        id="edit-date"
        v-model:value="editableEvent.date"
        type="date"
        value-format="yyyy-MM-dd"
      />
    </div>

    <div class="form-group">
      <label for="edit-location">地点:</label>
      <n-input id="edit-location" v-model:value="editableEvent.location" />
    </div>

    <div class="form-group">
      <label for="edit-vendor_password">摊主密码（可选）:</label>
      <n-input
        id="edit-vendor_password"
        v-model:value="editableEvent.vendor_password"
        type="password"
        placeholder="留空则保持原密码不变"
        show-password-on="click"
      />
      <small class="helper-text">
        输入新密码会覆盖旧密码，留空则不修改。
      </small>
    </div>

    <div class="qr-upload-row">
      <ImageUploader
        label="微信收款码"
        :initial-image-url="existingQrUrls[0]"
        v-model="newQrWechat"
        @image-removed="() => handleQrRemoved(0)"
        @invalid-file="handleInvalidFile"
      />
      <ImageUploader
        label="支付宝收款码"
        :initial-image-url="existingQrUrls[1]"
        v-model="newQrAlipay"
        @image-removed="() => handleQrRemoved(1)"
        @invalid-file="handleInvalidFile"
      />
    </div>

    <p v-if="errorMessage" class="error-message">{{ errorMessage }}</p>
  </n-form>
</template>

<script setup>
import { ref, watch, computed } from 'vue'
import { NDatePicker, NForm, NInput } from 'naive-ui'

import ImageUploader from '@/components/shared/ImageUploader.vue'

const props = defineProps({
  event: {
    type: Object,
    required: true,
  },
})

const errorMessage = ref('')
const editableEvent = ref({})
const newQrWechat = ref(null)
const newQrAlipay = ref(null)
const removedSlots = ref(new Set())

// 现有的收款码 URL 数组
const existingQrUrls = computed(() => {
  const urls = editableEvent.value?.qrcode_urls || []
  // 旧数据可能只有 qrcode_url 单个值
  if (urls.length === 0 && editableEvent.value?.qrcode_url) {
    return [editableEvent.value.qrcode_url]
  }
  return urls
})

watch(
  () => props.event,
  (newEvent) => {
    if (!newEvent) return
    editableEvent.value = { ...newEvent, vendor_password: newEvent.vendor_password || '' }
    if (!editableEvent.value.date) {
      editableEvent.value.date = null
    }
    newQrWechat.value = null
    newQrAlipay.value = null
    removedSlots.value = new Set()
    errorMessage.value = ''
  },
  { immediate: true }
)

function handleQrRemoved(index) {
  removedSlots.value.add(index)
}

function handleInvalidFile(message) {
  errorMessage.value = message
}

function submit() {
  if (!editableEvent.value.name || !editableEvent.value.date) {
    errorMessage.value = '展会名称和日期不能为空。'
    return null
  }

  errorMessage.value = ''
  const formData = new FormData()
  formData.append('id', editableEvent.value.id)
  formData.append('name', editableEvent.value.name)
  formData.append('date', editableEvent.value.date)
  formData.append('location', editableEvent.value.location || '')
  formData.append('vendor_password', editableEvent.value.vendor_password || '')

  const hasNewUpload = newQrWechat.value || newQrAlipay.value
  const allRemoved = removedSlots.value.size >= existingQrUrls.value.length && !hasNewUpload

  if (allRemoved) {
    // 所有码都被移除
    formData.append('remove_payment_qr_code', 'true')
  } else if (hasNewUpload) {
    // 有新上传：发送所有新码，后端会替换旧的
    if (newQrWechat.value) {
      formData.append('payment_qr_code_wechat', newQrWechat.value)
    }
    if (newQrAlipay.value) {
      formData.append('payment_qr_code_alipay', newQrAlipay.value)
    }
  }
  // 既没新上传也没全删除 → 保持原样，不发 payment 相关字段

  return formData
}

defineExpose({ submit })
</script>

<style scoped>
.form-group {
  margin-bottom: 1.5rem;
  display: flex;
  flex-direction: column;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 500;
  font-size: var(--font-md);
}

.helper-text {
  color: var(--text-muted);
  margin-top: 0.25rem;
  display: block;
}

.qr-upload-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  margin-bottom: 1rem;
}

@media (max-width: 600px) {
  .qr-upload-row {
    grid-template-columns: 1fr;
  }
}

.error-message {
  color: var(--error-color);
  font-size: var(--font-base);
  margin-top: 0.5rem;
}
</style>
