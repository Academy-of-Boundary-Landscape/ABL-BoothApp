<template>
  <div class="image-uploader-container">
    <label v-if="label" class="form-label">{{ label }}</label>

    <div class="image-preview-wrapper">
      <div v-if="previewUrl" class="image-preview-box" :style="boxStyle">
        <n-image
          :src="previewUrl"
          alt="新图片预览"
          class="image-preview"
          preview-disabled
          :width="maxWidth"
          :height="maxHeight"
        />
        <n-tag size="small" type="default" class="preview-tag">新图片</n-tag>
      </div>

      <div v-else-if="initialImageUrl" class="image-preview-box" :style="boxStyle">
        <n-image
          :src="displayInitialUrl"
          alt="当前图片"
          class="image-preview"
          preview-disabled
          :width="maxWidth"
          :height="maxHeight"
        />
        <n-tag size="small" type="default" class="preview-tag">当前图片</n-tag>
      </div>

      <div v-else class="no-image-placeholder">无图片</div>
    </div>

    <div class="image-actions">
      <n-upload
        accept="image/*"
        :default-upload="false"
        :show-file-list="false"
        @change="onUploadChange"
      >
        <n-button tertiary>
          {{ initialImageUrl || previewUrl ? '更换图片' : '选择图片' }}
        </n-button>
      </n-upload>

      <n-button
        v-if="initialImageUrl || previewUrl"
        type="error"
        tertiary
        @click="removeImage"
      >
        移除图片
      </n-button>
    </div>

    <p class="upload-hint">{{ uploadHint }}</p>
    <p v-if="errorMessage" class="upload-error">{{ errorMessage }}</p>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { NButton, NImage, NTag, NUpload } from 'naive-ui'

import { getImageUrl } from '@/services/url'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  IMAGE_WARN_THRESHOLD_MB,
  confirmLargeFile,
  showUploadDialog,
  validateFileSize,
} from '@/utils/upload'

const props = defineProps({
  modelValue: {
    type: File,
    default: null,
  },
  initialImageUrl: {
    type: String,
    default: '',
  },
  label: {
    type: String,
    default: '图片上传',
  },
  maxWidth: {
    type: Number,
    default: 200,
  },
  maxHeight: {
    type: Number,
    default: 200,
  },
  maxFileSizeMb: {
    type: Number,
    default: IMAGE_UPLOAD_LIMIT_MB,
  },
})

const emit = defineEmits(['update:modelValue', 'image-removed', 'invalid-file'])

const previewUrl = ref(null)
const errorMessage = ref('')

const displayInitialUrl = computed(() => getImageUrl(props.initialImageUrl))

const boxStyle = computed(() => ({
  width: `${props.maxWidth}px`,
  height: `${props.maxHeight}px`,
}))

const uploadHint = computed(
  () => `支持图片上传，单文件大小不超过 ${props.maxFileSizeMb}MB`
)

watch(
  () => props.initialImageUrl,
  () => {
    resetState()
  }
)

async function onUploadChange({ file }) {
  const raw = file?.file ?? null
  if (!raw) return

  const validation = validateFileSize(raw, props.maxFileSizeMb)
  if (!validation.ok) {
    resetState()
    errorMessage.value = validation.message
    emit('update:modelValue', null)
    emit('invalid-file', validation.message)
    showUploadDialog('上传文件过大', validation.message)
    return
  }

  // 文件超过阈值时弹出确认，用户可选择取消
  const fileSizeMb = raw.size / (1024 * 1024)
  if (fileSizeMb > IMAGE_WARN_THRESHOLD_MB) {
    const confirmed = await confirmLargeFile(fileSizeMb)
    if (!confirmed) return
  }

  errorMessage.value = ''
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value)
  }
  previewUrl.value = URL.createObjectURL(raw)
  emit('update:modelValue', raw)
}

function removeImage() {
  resetState()
  emit('update:modelValue', null)
  emit('image-removed')
}

function resetState() {
  if (previewUrl.value) {
    URL.revokeObjectURL(previewUrl.value)
  }
  previewUrl.value = null
  errorMessage.value = ''
}
</script>

<style scoped>
.image-uploader-container {
  margin-bottom: 1rem;
}

.form-label {
  display: block;
  margin-bottom: 0.5rem;
}

.image-preview-wrapper {
  margin-bottom: 1rem;
}

.image-preview-box {
  position: relative;
  display: inline-block;
  border: 1px solid var(--border-color);
  padding: 5px;
  border-radius: var(--radius-sm);
  background-color: var(--bg-color);
}

.image-preview {
  width: 100%;
  height: 100%;
  display: block;
}

.image-preview :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.preview-tag {
  position: absolute;
  top: 5px;
  left: 5px;
  background-color: var(--overlay-color);
  color: var(--text-white);
  padding: 2px 6px;
  font-size: var(--font-sm);
  border-radius: var(--radius-sm);
}

.no-image-placeholder {
  display: inline-block;
  padding: 2rem 3rem;
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-disabled);
}

.image-actions {
  display: flex;
  gap: 1rem;
}

.upload-hint {
  margin-top: 0.5rem;
  color: var(--text-disabled);
  font-size: var(--font-base);
}

.upload-error {
  margin-top: 0.25rem;
  color: var(--error-color);
  font-size: var(--font-base);
}
</style>
