<template>
  <n-form label-placement="top" @submit.prevent>
    <n-form-item label="展会名称" required>
      <n-input
        id="event-name"
        v-model:value="formData.name"
        placeholder="例如：COMICUP 31"
        :disabled="isSubmitting"
      />
    </n-form-item>

    <n-form-item label="日期" required>
      <n-date-picker
        id="event-date"
        v-model:formatted-value="formData.date"
        type="date"
        value-format="yyyy-MM-dd"
        :disabled="isSubmitting"
      />
    </n-form-item>

    <n-form-item label="地点">
      <n-input
        id="event-location"
        v-model:value="formData.location"
        placeholder="例如：上海"
        :disabled="isSubmitting"
      />
    </n-form-item>

    <n-form-item label="摊主密码">
      <n-input
        id="event-vendor-password"
        v-model:value="formData.vendorPassword"
        :type="isEdit ? 'password' : 'text'"
        :show-password-on="isEdit ? 'click' : undefined"
        :placeholder="isEdit ? '留空 = 不修改' : '留空则使用全局密码'"
        :disabled="isSubmitting"
      />
      <!-- 编辑时后端对空串保留原哈希（已核实），所以旁注要说明清楚，不能让摊主以为清空了。 -->
      <template v-if="isEdit" #feedback> 留空 = 不修改。输入新密码会覆盖旧密码。 </template>
    </n-form-item>

    <div class="qr-upload-row">
      <ImageUploader
        label="微信收款码（可选）"
        :initial-image-url="existingQrUrls[0]"
        v-model="newQrWechat"
        @image-removed="() => handleQrRemoved(0)"
        @invalid-file="handleInvalidFile"
      />
      <ImageUploader
        label="支付宝收款码（可选）"
        :initial-image-url="existingQrUrls[1]"
        v-model="newQrAlipay"
        @image-removed="() => handleQrRemoved(1)"
        @invalid-file="handleInvalidFile"
      />
    </div>

    <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
  </n-form>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NDatePicker, NForm, NFormItem, NInput } from 'naive-ui'

import type { Schemas } from '@/api/client'
import ImageUploader from '@/components/shared/ImageUploader.vue'
import { useEventStore } from '@/stores/eventStore'
import { IMAGE_UPLOAD_LIMIT_MB, normalizeUploadError } from '@/utils/upload'

type EventFormData = {
  name: string
  date: string | null
  location: string
  vendorPassword: string
}

const props = defineProps<{
  mode: 'create' | 'edit'
  event?: Schemas['EventResponse']
}>()

const emit = defineEmits<{
  (e: 'saved'): void
}>()

const store = useEventStore()

const isSubmitting = ref(false)
const errorMessage = ref('')
const formData = ref<EventFormData>({
  name: '',
  date: null,
  location: '',
  vendorPassword: '',
})
const newQrWechat = ref<File | undefined>(undefined)
const newQrAlipay = ref<File | undefined>(undefined)
const removedSlots = ref<Set<number>>(new Set())

const isEdit = computed(() => props.mode === 'edit')

// 现有的收款码 URL 数组（旧数据可能只有 qrcode_url 单个值）。
const existingQrUrls = computed(() => {
  if (!isEdit.value || !props.event) return []
  const urls = props.event.qrcode_urls || []
  if (urls.length === 0 && props.event.qrcode_url) {
    return [props.event.qrcode_url]
  }
  return urls
})

function hydrate() {
  const ev = props.event
  if (isEdit.value && ev) {
    formData.value = {
      name: ev.name,
      date: ev.date ?? null,
      location: ev.location ?? '',
      // 编辑模式初始密码恒为空：留空 = 不修改，绝不用旧值回填。
      vendorPassword: '',
    }
  } else {
    formData.value = { name: '', date: null, location: '', vendorPassword: '' }
  }
  newQrWechat.value = undefined
  newQrAlipay.value = undefined
  removedSlots.value = new Set()
  errorMessage.value = ''
}

watch(() => [props.mode, props.event] as const, hydrate, { immediate: true })

function handleQrRemoved(index: number) {
  removedSlots.value.add(index)
}

function handleInvalidFile(message: string) {
  errorMessage.value = message
}

function buildFormData(): FormData | null {
  if (!formData.value.name || !formData.value.date) {
    errorMessage.value = '展会名称和日期不能为空。'
    return null
  }
  if (isEdit.value && !props.event) {
    errorMessage.value = '缺少要编辑的展会。'
    return null
  }

  errorMessage.value = ''
  const formDataOut = new FormData()
  if (isEdit.value && props.event) {
    formDataOut.append('id', String(props.event.id))
  }
  formDataOut.append('name', formData.value.name)
  formDataOut.append('date', formData.value.date)
  formDataOut.append('location', formData.value.location || '')
  formDataOut.append('vendor_password', formData.value.vendorPassword || '')

  const hasNewUpload = Boolean(newQrWechat.value || newQrAlipay.value)
  if (isEdit.value) {
    const allRemoved = removedSlots.value.size >= existingQrUrls.value.length && !hasNewUpload
    if (allRemoved) {
      // 所有码都被移除。
      formDataOut.append('remove_payment_qr_code', 'true')
    } else if (hasNewUpload) {
      // 有新上传：发送所有新码，后端会替换旧的。
      if (newQrWechat.value) {
        formDataOut.append('payment_qr_code_wechat', newQrWechat.value)
      }
      if (newQrAlipay.value) {
        formDataOut.append('payment_qr_code_alipay', newQrAlipay.value)
      }
    }
    // 既没新上传也没全删除 → 保持原样，不发 payment 相关字段。
  } else {
    if (newQrWechat.value) {
      formDataOut.append('payment_qr_code_wechat', newQrWechat.value)
    }
    if (newQrAlipay.value) {
      formDataOut.append('payment_qr_code_alipay', newQrAlipay.value)
    }
  }

  return formDataOut
}

async function submit(): Promise<FormData | null> {
  const formDataOut = buildFormData()
  if (!formDataOut) return null

  isSubmitting.value = true
  try {
    if (isEdit.value) {
      if (!props.event) return null
      await store.updateEvent(props.event.id, formDataOut)
    } else {
      await store.createEvent(formDataOut)
      hydrate()
    }
    emit('saved')
    return formDataOut
  } catch (error) {
    errorMessage.value = normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB)
    return null
  } finally {
    isSubmitting.value = false
  }
}

defineExpose({ submit, buildFormData })
</script>

<style scoped>
.qr-upload-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
}

/* 编辑时表单里出现密码框，不需要额外的间距 hack；字段之间由 n-form-item 自身留白。 */
.form-error {
  color: var(--error-color);
  font-size: var(--font-base);
  margin-top: var(--space-sm);
}

@media (--phone) {
  .qr-upload-row {
    grid-template-columns: 1fr;
  }
}
</style>
