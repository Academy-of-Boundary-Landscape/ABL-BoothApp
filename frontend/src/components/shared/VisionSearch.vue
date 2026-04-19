<template>
  <div class="vision-search">
    <!-- ========== 摄像头取景模式 ========== -->
    <div v-if="cameraMode && isCameraActive" class="vision-camera">
      <div ref="viewportRef" class="vision-camera__viewport">
        <video
          ref="videoRef"
          autoplay
          playsinline
          muted
          class="vision-camera__video"
          :class="{ 'vision-camera__video--mirrored': currentFacing === 'user' }"
        />
        <!-- 取景框遮罩 -->
        <div class="vision-camera__overlay">
          <div class="vision-camera__frame" :style="frameStyle">
            <span class="frame-corner frame-corner--tl" />
            <span class="frame-corner frame-corner--tr" />
            <span class="frame-corner frame-corner--bl" />
            <span class="frame-corner frame-corner--br" />
          </div>
          <div class="vision-camera__hint">{{ isSearching ? '识别中...' : '将商品对准取景框' }}</div>
        </div>

        <!-- 拍照闪光 -->
        <div v-if="showFlash" class="vision-camera__flash" />

        <!-- 搜索中 loading -->
        <div v-if="isSearching" class="vision-camera__loading">
          <div class="loading-spinner" />
          <span class="loading-text">正在识别</span>
        </div>
      </div>

      <div class="vision-camera__controls">
        <n-button tertiary size="small" @click="stopCamera">取消</n-button>
        <button
          class="vision-camera__shutter"
          :disabled="isSearching"
          @click="captureAndSearch"
        >
          <span class="vision-camera__shutter-inner" />
        </button>
        <n-button tertiary size="small" @click="switchCamera">翻转</n-button>
      </div>

      <!-- 用于截图的隐藏 canvas -->
      <canvas ref="canvasRef" class="vision-camera__canvas" />
    </div>

    <!-- ========== 普通输入模式 ========== -->
    <template v-else>
      <div class="vision-input">
        <div
          class="vision-dropzone"
          :class="{ 'vision-dropzone--active': isDragging, 'vision-dropzone--has-image': previewUrl }"
          @dragover.prevent="isDragging = true"
          @dragleave.prevent="isDragging = false"
          @drop.prevent="onDrop"
          @click="triggerFileInput"
        >
          <img v-if="previewUrl" :src="previewUrl" class="vision-dropzone__preview" alt="查询图片" />
          <div v-else class="vision-dropzone__placeholder">
            <span class="vision-dropzone__icon">+</span>
            <span class="vision-dropzone__text">拍照 / 拖入图片</span>
          </div>
        </div>
        <input
          ref="fileInputRef"
          type="file"
          accept="image/*"
          capture="environment"
          class="vision-file-input"
          @change="onFileSelected"
        />
        <div class="vision-input-actions">
          <n-button
            v-if="cameraMode"
            size="small"
            tertiary
            @click="startCamera"
          >
            打开摄像头
          </n-button>
          <n-button
            v-if="previewUrl"
            size="small"
            tertiary
            @click.stop="clearImage"
          >
            清除
          </n-button>
        </div>
      </div>

      <!-- 搜索按钮 -->
      <n-button
        type="primary"
        :disabled="!selectedFile || isSearching"
        :loading="isSearching"
        block
        @click="doSearch"
      >
        {{ isSearching ? '搜索中...' : '以图搜图' }}
      </n-button>
    </template>

    <!-- ========== 搜索结果：摄像头模式用居中悬浮弹窗，普通模式用内联列表 ========== -->

    <!-- 摄像头模式：悬浮弹窗 -->
    <Transition name="result-pop">
      <div v-if="results && results.length && cameraMode && isCameraActive" class="vision-popup-backdrop" @click.self="results = []">
        <div class="vision-popup">
          <div class="vision-popup__header">
            <span>匹配结果</span>
            <n-tag v-if="isUncertain" size="small" type="warning">置信度较低</n-tag>
          </div>
          <div class="vision-popup__list">
            <div
              v-for="item in results"
              :key="item.master_product_id"
              class="vision-result-item"
              @click="onResultClick(item); results = []"
            >
              <div class="vision-result-item__thumb">
                <img v-if="item.thumb_url" :src="resolveThumb(item.thumb_url)" alt="" />
                <div v-else class="vision-result-item__no-thumb">?</div>
              </div>
              <div class="vision-result-item__info">
                <div class="vision-result-item__name">{{ item.name }}</div>
                <div class="vision-result-item__code">{{ item.product_code }}</div>
              </div>
              <div class="vision-result-item__score">
                {{ (item.score * 100).toFixed(1) }}%
              </div>
            </div>
          </div>
          <div class="vision-popup__footer">
            <n-button size="small" tertiary @click="results = []">关闭</n-button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 普通模式：内联列表 -->
    <div v-if="results && results.length && !(cameraMode && isCameraActive)" class="vision-results">
      <div class="vision-results__header">
        <span>匹配结果</span>
        <n-tag v-if="isUncertain" size="small" type="warning">置信度较低</n-tag>
      </div>
      <div
        v-for="item in results"
        :key="item.master_product_id"
        class="vision-result-item"
        :class="{ 'vision-result-item--selected': isItemSelected(item) }"
        @click="onResultClick(item)"
      >
        <div class="vision-result-item__thumb">
          <img v-if="item.thumb_url" :src="resolveThumb(item.thumb_url)" alt="" />
          <div v-else class="vision-result-item__no-thumb">?</div>
        </div>
        <div class="vision-result-item__info">
          <div class="vision-result-item__name">{{ item.name }}</div>
          <div class="vision-result-item__code">{{ item.product_code }}</div>
        </div>
        <div class="vision-result-item__score">
          {{ (item.score * 100).toFixed(1) }}%
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { NButton, NTag } from 'naive-ui'

import { searchByImage } from '@/services/vision'
import { getImageUrl } from '@/services/url'
import { useAlert } from '@/services/useAlert'
import { resizeImageFile } from '@/utils/upload'

const props = defineProps({
  mode: { type: String, default: null },
  eventId: { type: Number, default: null },
  masterProductIds: { type: Array, default: () => [] },
  topK: { type: Number, default: 5 },
  multiSelect: { type: Boolean, default: false },
  /** 启用摄像头取景模式 */
  cameraMode: { type: Boolean, default: false },
  /** 默认摄像头方向: "user"(前置) | "environment"(后置) */
  facingMode: { type: String, default: 'user' },
})

const emit = defineEmits(['select', 'search-done', 'search-error'])

// ===================== 图片输入（文件模式）=====================
const fileInputRef = ref(null)
const selectedFile = ref(null)
const previewUrl = ref(null)
const isDragging = ref(false)

function triggerFileInput() {
  fileInputRef.value?.click()
}

function onFileSelected(e) {
  const file = e.target.files?.[0]
  if (file) setImage(file)
  e.target.value = ''
}

function onDrop(e) {
  isDragging.value = false
  const file = e.dataTransfer?.files?.[0]
  if (file && file.type.startsWith('image/')) setImage(file)
}

const MAX_VISION_SIZE = 512

async function setImage(file) {
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  const compressed = await resizeImageFile(file, MAX_VISION_SIZE)
  selectedFile.value = compressed
  previewUrl.value = URL.createObjectURL(compressed)
}

function clearImage() {
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  selectedFile.value = null
  previewUrl.value = null
  results.value = []
  errorMsg.value = ''
}

// ===================== 摄像头模式 =====================
const videoRef = ref(null)
const viewportRef = ref(null)
const canvasRef = ref(null)
const isCameraActive = ref(false)
const currentStream = ref(null)
const currentFacing = ref(props.facingMode)

// 取景框：占 viewport 短边的 65%，正方形，居中
const FRAME_RATIO = 0.65
const vpSize = ref({ w: 1, h: 1 })

function updateVpSize() {
  const el = viewportRef.value
  if (!el) return
  vpSize.value = { w: el.clientWidth || 1, h: el.clientHeight || 1 }
}

// CSS 定位：像素级正方形
const frameStyle = computed(() => {
  const { w: vpW, h: vpH } = vpSize.value
  const shortSide = Math.min(vpW, vpH)
  const size = shortSide * FRAME_RATIO
  return {
    width: `${size}px`,
    height: `${size}px`,
  }
})

async function startCamera() {
  errorMsg.value = ''
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: currentFacing.value, width: { ideal: 1280 }, height: { ideal: 960 } },
      audio: false,
    })
    currentStream.value = stream
    isCameraActive.value = true

    // 等待 DOM 更新后绑定 video 并测量 viewport
    await nextTick()
    if (videoRef.value) {
      videoRef.value.srcObject = stream
    }
    updateVpSize()
  } catch (err) {
    errorMsg.value = '无法访问摄像头: ' + (err.message || err.name)
  }
}

function stopCamera() {
  if (currentStream.value) {
    currentStream.value.getTracks().forEach((t) => t.stop())
    currentStream.value = null
  }
  isCameraActive.value = false
}

async function switchCamera() {
  currentFacing.value = currentFacing.value === 'user' ? 'environment' : 'user'
  stopCamera()
  await startCamera()
}

function captureFrame() {
  const video = videoRef.value
  const viewport = viewportRef.value
  const canvas = canvasRef.value
  if (!video || !canvas || !viewport) return null

  const vw = video.videoWidth
  const vh = video.videoHeight
  if (!vw || !vh) return null

  // 计算 object-fit: cover 的实际裁切区域
  const vpRect = viewport.getBoundingClientRect()
  const vpW = vpRect.width
  const vpH = vpRect.height

  const videoAspect = vw / vh
  const vpAspect = vpW / vpH

  let srcX = 0, srcY = 0, srcW = vw, srcH = vh
  if (videoAspect > vpAspect) {
    // 视频比 viewport 更宽，左右被裁
    srcW = vh * vpAspect
    srcX = (vw - srcW) / 2
  } else {
    // 视频比 viewport 更高，上下被裁
    srcH = vw / vpAspect
    srcY = (vh - srcH) / 2
  }

  // 取景框在可见区域中的位置（基于短边的正方形，居中）
  const shortSrc = Math.min(srcW, srcH)
  const frameSide = shortSrc * FRAME_RATIO
  const frameX = srcX + (srcW - frameSide) / 2
  const frameY = srcY + (srcH - frameSide) / 2
  const frameW = frameSide
  const frameH = frameSide

  // 前置摄像头镜像：水平翻转 x 坐标
  let finalX = frameX
  if (currentFacing.value === 'user') {
    finalX = vw - frameX - frameW
  }

  // 输出正方形图像
  const outSize = Math.round(Math.max(frameW, frameH))
  canvas.width = outSize
  canvas.height = outSize
  const ctx = canvas.getContext('2d')
  ctx.drawImage(video, finalX, frameY, frameW, frameH, 0, 0, outSize, outSize)

  return new Promise((resolve) => {
    canvas.toBlob((blob) => resolve(blob), 'image/jpeg', 0.92)
  })
}

const showFlash = ref(false)

async function captureAndSearch() {
  // 闪光反馈
  showFlash.value = true
  setTimeout(() => { showFlash.value = false }, 200)

  const blob = await captureFrame()
  if (!blob) return

  // 压缩到 512×512 后再发送
  const raw = new File([blob], 'capture.jpg', { type: 'image/jpeg' })
  const compressed = await resizeImageFile(raw, MAX_VISION_SIZE)

  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  previewUrl.value = URL.createObjectURL(compressed)
  selectedFile.value = compressed

  await doSearch()
}

// ===================== 搜索 =====================
const isSearching = ref(false)
const results = ref([])
const isUncertain = ref(false)
const errorMsg = ref('')
const selectedIds = ref(new Set())

const VISION_ERROR_MAP = {
  'VISION_NOT_READY': 'AI 视觉识别尚未就绪，请先在管理后台安装模型并构建索引',
  'VISION_REBUILDING': 'AI 索引正在构建中，请稍后再试',
  'VISION_BUSY': '识别请求过多，请稍后再试',
  'VISION_TIMEOUT': '识别超时，请重试',
}

function translateVisionError(code) {
  return VISION_ERROR_MAP[code] || null
}

async function doSearch() {
  if (!selectedFile.value) return

  isSearching.value = true
  errorMsg.value = ''
  // 先清空旧结果，等一个渲染帧再发请求
  // 确保 Transition 能正确检测到 "无结果 → 有结果" 的变化
  results.value = []
  selectedIds.value.clear()
  await nextTick()

  try {
    const resp = await searchByImage(selectedFile.value, {
      topK: props.topK,
      mode: props.mode,
      eventId: props.eventId,
      masterProductIds: props.masterProductIds.length ? props.masterProductIds : undefined,
    })
    results.value = resp.results || []
    isUncertain.value = resp.is_uncertain ?? false
    emit('search-done', resp)
  } catch (err) {
    const raw = err.response?.data?.error || err.response?.data || ''
    const msg = translateVisionError(typeof raw === 'string' ? raw : '')
      || (err.code === 'ECONNABORTED' ? '搜索超时，请重试' : '搜索失败')
    errorMsg.value = msg
    const { showError } = useAlert()
    showError(msg)
    emit('search-error', msg)
  } finally {
    isSearching.value = false
  }
}

// ===================== 结果交互 =====================
function isItemSelected(item) {
  return selectedIds.value.has(item.master_product_id)
}

function onResultClick(item) {
  if (props.multiSelect) {
    const ids = selectedIds.value
    if (ids.has(item.master_product_id)) ids.delete(item.master_product_id)
    else ids.add(item.master_product_id)
  } else {
    selectedIds.value = new Set([item.master_product_id])
  }
  emit('select', item)
}

function resolveThumb(url) {
  return getImageUrl(url)
}

// ===================== 生命周期 =====================
let resizeObs = null

onMounted(() => {
  if (props.cameraMode) startCamera()
  // 监听 viewport 尺寸变化，保持取景框正方形
  resizeObs = new ResizeObserver(updateVpSize)
  if (viewportRef.value) resizeObs.observe(viewportRef.value)
})

watch(
  () => [props.mode, props.eventId, props.masterProductIds],
  () => { results.value = []; errorMsg.value = '' }
)

// viewport ref 可能在 camera 打开后才出现
watch(viewportRef, (el) => {
  if (el && resizeObs) resizeObs.observe(el)
})

onBeforeUnmount(() => {
  stopCamera()
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  if (resizeObs) resizeObs.disconnect()
})

</script>

<style scoped>
.vision-search {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  position: relative;
}

/* ===================== 摄像头取景 ===================== */
.vision-camera {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.vision-camera__viewport {
  position: relative;
  border-radius: var(--radius-md);
  overflow: hidden;
  background: #000;
  flex: 1;
  min-height: 0;
}

.vision-camera__video {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.vision-camera__video--mirrored {
  transform: scaleX(-1);
}

.vision-camera__overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

/* 取景框：box-shadow 实现框外暗化 */
.vision-camera__frame {
  position: relative;
  border-radius: 12px;
  box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.5);
}

/* 拍照闪光 */
.vision-camera__flash {
  position: absolute;
  inset: 0;
  background: white;
  z-index: 20;
  animation: flash-fade 0.2s ease-out forwards;
  pointer-events: none;
}
@keyframes flash-fade {
  0% { opacity: 0.85; }
  100% { opacity: 0; }
}

/* 搜索中 loading */
.vision-camera__loading {
  position: absolute;
  inset: 0;
  z-index: 15;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  background: rgba(0, 0, 0, 0.4);
  pointer-events: none;
}
.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.loading-text {
  color: white;
  font-size: var(--font-md);
  font-weight: 600;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
}

.frame-corner {
  position: absolute;
  width: 28px;
  height: 28px;
  border-color: #fff;
  border-style: solid;
}

.frame-corner--tl {
  top: -2px; left: -2px;
  border-width: 4px 0 0 4px;
  border-radius: 8px 0 0 0;
}

.frame-corner--tr {
  top: -2px; right: -2px;
  border-width: 4px 4px 0 0;
  border-radius: 0 8px 0 0;
}

.frame-corner--bl {
  bottom: -2px; left: -2px;
  border-width: 0 0 4px 4px;
  border-radius: 0 0 0 8px;
}

.frame-corner--br {
  bottom: -2px; right: -2px;
  border-width: 0 4px 4px 0;
  border-radius: 0 0 8px 0;
}

.vision-camera__hint {
  margin-top: 16px;
  color: rgba(255, 255, 255, 0.85);
  font-size: var(--font-md);
  font-weight: 600;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  letter-spacing: 0.05em;
}

/* 控制栏 */
.vision-camera__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
}

.vision-camera__shutter {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  border: 4px solid var(--accent-color);
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.15s;
}

.vision-camera__shutter:active {
  opacity: 0.7;
}

.vision-camera__shutter:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.vision-camera__shutter-inner {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--accent-color);
  display: block;
  transition: transform 0.1s;
}

.vision-camera__shutter:active .vision-camera__shutter-inner {
  transform: scale(0.9);
}

.vision-camera__canvas {
  display: none;
}

/* ===================== 文件输入 ===================== */
.vision-input {
  position: relative;
}

.vision-file-input {
  display: none;
}

.vision-dropzone {
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-md);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 140px;
  transition: border-color 0.2s, background-color 0.2s;
  overflow: hidden;
  background: var(--bg-color);
}

.vision-dropzone:hover,
.vision-dropzone--active {
  border-color: var(--accent-color);
  background: var(--hover-bg-color);
}

.vision-dropzone--has-image {
  border-style: solid;
  padding: 4px;
}

.vision-dropzone__preview {
  max-width: 100%;
  max-height: 200px;
  object-fit: contain;
  border-radius: var(--radius-sm);
}

.vision-dropzone__placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--text-disabled);
}

.vision-dropzone__icon {
  font-size: var(--font-2xl);
  line-height: 1;
}

.vision-dropzone__text {
  font-size: var(--font-base);
}

.vision-input-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

/* ===================== 错误 ===================== */
.vision-error {
  color: var(--error-color);
  font-size: var(--font-base);
  padding: 6px 0;
}

/* ===================== 结果列表 ===================== */
.vision-results__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 600;
  font-size: var(--font-base);
  color: var(--primary-text-color);
  margin-bottom: 4px;
}

.vision-result-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background-color 0.15s;
  border: 1px solid transparent;
}

.vision-result-item:hover {
  background: var(--hover-bg-color);
}

.vision-result-item--selected {
  border-color: var(--accent-color);
  background: var(--hover-bg-color);
}

.vision-result-item__thumb {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.vision-result-item__thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.vision-result-item__no-thumb {
  color: var(--text-disabled);
  font-size: var(--font-lg);
}

.vision-result-item__info {
  flex: 1;
  min-width: 0;
}

.vision-result-item__name {
  font-size: var(--font-base);
  color: var(--primary-text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.vision-result-item__code {
  font-size: var(--font-sm);
  color: var(--text-disabled);
}

.vision-result-item__score {
  flex-shrink: 0;
  font-size: var(--font-base);
  font-weight: 600;
  color: var(--accent-color);
  min-width: 50px;
  text-align: right;
}

/* 摄像头模式：居中悬浮弹窗 */
.vision-popup-backdrop {
  position: fixed;
  inset: 0;
  z-index: 5000;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.vision-popup {
  background: var(--card-bg-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-xl);
  width: 100%;
  max-width: 420px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.vision-popup__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  font-weight: 700;
  font-size: var(--font-md);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.vision-popup__list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
}

.vision-popup__footer {
  padding: 10px 16px;
  border-top: 1px solid var(--border-color);
  text-align: center;
  flex-shrink: 0;
}

/* 弹窗动画 */
.result-pop-enter-active { transition: opacity 0.2s, transform 0.2s; }
.result-pop-leave-active { transition: opacity 0.15s, transform 0.15s; }
.result-pop-enter-from { opacity: 0; transform: scale(0.95); }
.result-pop-leave-to { opacity: 0; transform: scale(0.95); }
</style>
