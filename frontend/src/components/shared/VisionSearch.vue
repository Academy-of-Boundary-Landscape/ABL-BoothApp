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
          <div class="vision-camera__hint">
            {{ isSearching ? '识别中...' : '将商品对准取景框' }}
          </div>
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
        <button class="vision-camera__shutter" :disabled="isSearching" @click="captureAndSearch">
          <span class="vision-camera__shutter-inner" />
        </button>
        <n-button tertiary size="small" @click="switchCamera">翻转</n-button>
      </div>

      <!-- 用于截图的隐藏 canvas -->
      <canvas ref="canvasRef" class="vision-camera__canvas" />
    </div>

    <!-- ========== 相机模式：未激活态（取景框启动前/被关闭后的过渡页） ========== -->
    <template v-else-if="cameraMode">
      <div class="vision-camera-cta">
        <div class="vision-camera-cta__icon">📷</div>
        <div class="vision-camera-cta__title">拍照识别商品</div>
        <div v-if="errorMsg" class="vision-camera-cta__error">{{ errorMsg }}</div>
        <div v-else class="vision-camera-cta__hint">
          点击下方按钮开启摄像头，对准商品按快门即可识别
        </div>
        <n-button
          type="primary"
          size="large"
          block
          class="vision-camera-cta__primary"
          @click="startCamera"
        >
          📷 开启摄像头
        </n-button>
        <n-button
          size="small"
          tertiary
          block
          class="vision-camera-cta__fallback"
          @click="triggerFileInput"
        >
          或从相册选择图片
        </n-button>
        <input
          ref="fileInputRef"
          type="file"
          accept="image/*"
          capture="environment"
          class="vision-file-input"
          @change="onFileSelected"
        />
        <!-- 若用户从相册选了图片，展示预览 + 搜索按钮 -->
        <div v-if="previewUrl" class="vision-camera-cta__preview">
          <img :src="previewUrl" class="vision-camera-cta__preview-img" alt="查询图片" />
          <div class="vision-camera-cta__preview-actions">
            <n-button size="small" tertiary @click="clearImage">清除</n-button>
            <n-button
              type="primary"
              size="small"
              :loading="isSearching"
              :disabled="isSearching"
              @click="doSearch"
            >
              {{ isSearching ? '搜索中...' : '以图搜图' }}
            </n-button>
          </div>
        </div>
      </div>
    </template>

    <!-- ========== 普通输入模式（非相机场景，如管理端上传） ========== -->
    <template v-else>
      <div class="vision-input">
        <div
          class="vision-dropzone"
          :class="{
            'vision-dropzone--active': isDragging,
            'vision-dropzone--has-image': previewUrl,
          }"
          @dragover.prevent="isDragging = true"
          @dragleave.prevent="isDragging = false"
          @drop.prevent="onDrop"
          @click="triggerFileInput"
        >
          <img
            v-if="previewUrl"
            :src="previewUrl"
            class="vision-dropzone__preview"
            alt="查询图片"
          />
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
          <n-button v-if="previewUrl" size="small" tertiary @click.stop="clearImage">
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
    <AppModal
      :show="showCameraResults"
      size="sm"
      :closable="false"
      @update:show="onCameraResultsShow"
    >
      <template #header>
        <span class="vision-popup__title">匹配结果</span>
        <n-tag v-if="isUncertain" size="small" type="warning">置信度较低</n-tag>
      </template>
      <div class="vision-popup__list">
        <div
          v-for="item in results"
          :key="item.master_product_id"
          class="vision-result-item"
          @click="selectResultAndClose(item)"
        >
          <div class="vision-result-item__thumb">
            <img v-if="item.thumb_url" :src="resolveThumb(item.thumb_url)" alt="" />
            <div v-else class="vision-result-item__no-thumb">?</div>
          </div>
          <div class="vision-result-item__info">
            <div class="vision-result-item__name">{{ item.name }}</div>
            <div class="vision-result-item__code">{{ item.product_code }}</div>
          </div>
          <div class="vision-result-item__score">{{ (item.score * 100).toFixed(1) }}%</div>
        </div>
      </div>
      <template #footer>
        <n-button size="small" tertiary @click="results = []">关闭</n-button>
      </template>
    </AppModal>

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
        <div class="vision-result-item__score">{{ (item.score * 100).toFixed(1) }}%</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { NButton, NTag } from 'naive-ui'

import { AppModal } from '@/components/ui'
import { searchByImage } from '@/services/vision'
import { getImageUrl } from '@/services/url'
import { useFeedback } from '@/composables/useFeedback'
import { resizeImageFile } from '@/utils/upload'
import { ApiRequestError, errorMessage, type Schemas } from '@/api/client'

interface Props {
  mode?: string | null
  eventId?: number | null
  masterProductIds?: number[]
  topK?: number
  multiSelect?: boolean
  /** 启用摄像头取景模式 */
  cameraMode?: boolean
  /** 默认摄像头方向: "user"(前置) | "environment"(后置) */
  facingMode?: string
}

const props = withDefaults(defineProps<Props>(), {
  mode: null,
  eventId: null,
  masterProductIds: () => [],
  topK: 5,
  multiSelect: false,
  cameraMode: false,
  facingMode: 'user',
})

const emit = defineEmits<{
  (e: 'select', item: Schemas['VisionSearchResult']): void
  (e: 'search-done', resp: Schemas['VisionSearchResponse']): void
  (e: 'search-error', msg: string): void
}>()

const fb = useFeedback()

// ===================== 图片输入（文件模式）=====================
const fileInputRef = ref<HTMLInputElement | null>(null)
const selectedFile = ref<File | Blob | null>(null)
const previewUrl = ref<string | null>(null)
const isDragging = ref(false)

function triggerFileInput() {
  fileInputRef.value?.click()
}

function onFileSelected(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) setImage(file)
  input.value = ''
}

function onDrop(e: DragEvent) {
  isDragging.value = false
  const file = e.dataTransfer?.files?.[0]
  if (file && file.type.startsWith('image/')) setImage(file)
}

const MAX_VISION_SIZE = 512

async function setImage(file: File | Blob) {
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
const videoRef = ref<HTMLVideoElement | null>(null)
const viewportRef = ref<HTMLDivElement | null>(null)
const canvasRef = ref<HTMLCanvasElement | null>(null)
const isCameraActive = ref(false)
const currentStream = ref<MediaStream | null>(null)
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

  // 防御性兜底：getUserMedia 仅在 secure context（HTTPS / localhost）可用。
  // 摊主在 LAN 浏览器首次访问 https URL 但未接受证书时，
  // 或意外通过 http URL 进入时，给清晰提示而不是浏览器内部错误。
  if (
    !window.isSecureContext ||
    !navigator.mediaDevices ||
    typeof navigator.mediaDevices.getUserMedia !== 'function'
  ) {
    errorMsg.value =
      '当前页面不是安全连接，浏览器禁止访问摄像头。请确认 URL 以 https 开头，' +
      '且首次访问时已点击「高级 → 继续访问」接受证书。' +
      '如仍无法解决，请直接在主机的摊盒桌面应用内拍照。'
    return
  }

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
    const e = err as { message?: string; name?: string }
    errorMsg.value = '无法访问摄像头: ' + (e.message || e.name)
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

function captureFrame(): Promise<Blob | null> | null {
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

  let srcX = 0,
    srcY = 0,
    srcW = vw,
    srcH = vh
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
  // canvas 一定支持 2d context；这里沿用旧行为（null 时抛错），不做静默降级。
  ctx!.drawImage(video, finalX, frameY, frameW, frameH, 0, 0, outSize, outSize)

  return new Promise((resolve) => {
    canvas.toBlob((blob) => resolve(blob), 'image/jpeg', 0.92)
  })
}

const showFlash = ref(false)

async function captureAndSearch() {
  // 闪光反馈
  showFlash.value = true
  setTimeout(() => {
    showFlash.value = false
  }, 200)

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
const results = ref<Schemas['VisionSearchResult'][]>([])
const isUncertain = ref(false)
const errorMsg = ref('')
const selectedIds = ref(new Set<number>())

const showCameraResults = computed(() =>
  Boolean(results.value.length && props.cameraMode && isCameraActive.value)
)

function onCameraResultsShow(v: boolean) {
  if (!v) results.value = []
}

const VISION_ERROR_MAP: Record<string, string> = {
  VISION_NOT_READY: 'AI 视觉识别尚未就绪，请先在管理后台安装模型并构建索引',
  VISION_REBUILDING: 'AI 索引正在构建中，请稍后再试',
  VISION_BUSY: '识别请求过多，请稍后再试',
  VISION_TIMEOUT: '识别超时，请重试',
}

function translateVisionError(code: string) {
  return VISION_ERROR_MAP[code] || null
}

async function doSearch() {
  const file = selectedFile.value
  if (!file) return

  isSearching.value = true
  errorMsg.value = ''
  // 先清空旧结果，等一个渲染帧再发请求
  // 确保 Transition 能正确检测到 "无结果 → 有结果" 的变化
  results.value = []
  selectedIds.value.clear()
  await nextTick()

  try {
    const resp = await searchByImage(file, {
      topK: props.topK,
      mode: props.mode ?? undefined,
      eventId: props.eventId ?? undefined,
      masterProductIds: props.masterProductIds.length ? props.masterProductIds : undefined,
    })
    results.value = resp.results || []
    isUncertain.value = resp.is_uncertain ?? false
    emit('search-done', resp)
  } catch (err) {
    // 新 client 把后端 {"error": "..."} 收进 ApiRequestError.serverMessage；超时/网络错误
    // 由 unwrap 统一包装成 status 0（旧 axios 的 ECONNABORTED 不再存在）。
    const raw = errorMessage(err, '')
    const isTimeout = err instanceof ApiRequestError && err.message === '请求超时'
    const msg = translateVisionError(raw) || (isTimeout ? '搜索超时，请重试' : '搜索失败')
    errorMsg.value = msg
    fb.alert({ title: '错误', content: msg, type: 'error' })
    emit('search-error', msg)
  } finally {
    isSearching.value = false
  }
}

// ===================== 结果交互 =====================
function isItemSelected(item: Schemas['VisionSearchResult']) {
  return selectedIds.value.has(item.master_product_id)
}

function onResultClick(item: Schemas['VisionSearchResult']) {
  if (props.multiSelect) {
    const ids = selectedIds.value
    if (ids.has(item.master_product_id)) ids.delete(item.master_product_id)
    else ids.add(item.master_product_id)
  } else {
    selectedIds.value = new Set([item.master_product_id])
  }
  emit('select', item)
}

// 摄像头悬浮结果弹窗专用：选中后顺带关掉弹窗。
// 原来是内联在模板里的 `onResultClick(item); results = []`，但 prettier
// 用 semi:false 重排多语句内联处理器时会吞掉分号，导致 Vue 编译器解析
// 报错（构建直接失败），所以拆成命名函数，行为不变。
function selectResultAndClose(item: Schemas['VisionSearchResult']) {
  onResultClick(item)
  results.value = []
}

function resolveThumb(url: string) {
  return getImageUrl(url)
}

// ===================== 生命周期 =====================
let resizeObs: ResizeObserver | null = null

onMounted(() => {
  if (props.cameraMode) startCamera()
  // 监听 viewport 尺寸变化，保持取景框正方形
  resizeObs = new ResizeObserver(updateVpSize)
  if (viewportRef.value) resizeObs.observe(viewportRef.value)
})

watch(
  () => [props.mode, props.eventId, props.masterProductIds],
  () => {
    results.value = []
    errorMsg.value = ''
  }
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
  gap: var(--space-md);
  height: 100%;
  min-height: 0;
  position: relative;
}

/* ===================== 摄像头取景 ===================== */
.vision-camera {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
  min-height: 0;
}

.vision-camera__viewport {
  position: relative;
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--overlay-color);
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
  border-radius: var(--radius-lg);
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 摄像头取景框：用超大 spread 阴影实现框外遮罩，非通用阴影 */
  box-shadow: 0 0 0 9999px var(--overlay-color);
}

/* 拍照闪光 */
.vision-camera__flash {
  position: absolute;
  inset: 0;
  background: var(--text-white);
  z-index: 20;
  animation: flash-fade 0.2s ease-out forwards;
  pointer-events: none;
}
@keyframes flash-fade {
  0% {
    opacity: 0.85;
  }
  100% {
    opacity: 0;
  }
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
  gap: var(--space-md);
  background: var(--overlay-color);
  pointer-events: none;
}
.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid color-mix(in srgb, var(--text-white) 30%, transparent);
  border-top-color: var(--text-white);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.loading-text {
  color: var(--text-white);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  text-shadow: 0 1px 3px var(--overlay-color);
}

.frame-corner {
  position: absolute;
  width: 28px;
  height: 28px;
  border-color: var(--text-white);
  border-style: solid;
}

.frame-corner--tl {
  top: -2px;
  left: -2px;
  border-width: 4px 0 0 4px;
  border-radius: var(--radius-md) 0 0 0;
}

.frame-corner--tr {
  top: -2px;
  right: -2px;
  border-width: 4px 4px 0 0;
  border-radius: 0 var(--radius-md) 0 0;
}

.frame-corner--bl {
  bottom: -2px;
  left: -2px;
  border-width: 0 0 4px 4px;
  border-radius: 0 0 0 var(--radius-md);
}

.frame-corner--br {
  bottom: -2px;
  right: -2px;
  border-width: 0 4px 4px 0;
  border-radius: 0 0 var(--radius-md) 0;
}

.vision-camera__hint {
  margin-top: var(--space-lg);
  color: color-mix(in srgb, var(--text-white) 85%, transparent);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  text-shadow: 0 1px 4px var(--overlay-color);
  letter-spacing: 0.05em;
}

/* 控制栏 */
.vision-camera__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-md);
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
  transition:
    border-color 0.2s,
    background-color 0.2s;
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
  padding: var(--space-xs);
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
  gap: var(--space-sm);
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
  gap: var(--space-sm);
  margin-top: var(--space-sm);
}

/* ===================== 错误 ===================== */
.vision-error {
  color: var(--error-color);
  font-size: var(--font-base);
  padding: var(--space-sm) 0;
}

/* ===================== 结果列表 ===================== */
.vision-results__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: var(--weight-bold);
  font-size: var(--font-base);
  color: var(--primary-text-color);
  margin-bottom: var(--space-xs);
}

.vision-result-item {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm);
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
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  min-width: 50px;
  text-align: right;
}

/* 摄像头模式：结果列表（弹窗外壳由 AppModal 提供） */
.vision-popup__title {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
}

.vision-popup__list {
  max-height: 70vh;
  overflow-y: auto;
  padding: var(--space-xs) var(--space-sm);
}

/* ========== 相机模式未激活时的 CTA 过渡页 ========== */
.vision-camera-cta {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: var(--space-md);
  padding: var(--space-2xl) var(--space-lg);
  text-align: center;
  background: var(--card-bg-color);
  border: 1.5px solid var(--border-color);
  border-radius: var(--radius-md);
}
.vision-camera-cta__icon {
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 装饰性 emoji 图标：降到 --font-2xl 会明显变小影响观感 */
  font-size: 3rem;
  line-height: 1;
  margin-bottom: var(--space-xs);
}
.vision-camera-cta__title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}
.vision-camera-cta__hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
  line-height: 1.5;
  max-width: 320px;
  margin: 0 auto var(--space-sm);
}
.vision-camera-cta__error {
  font-size: var(--font-sm);
  color: var(--error-color);
  line-height: 1.5;
  padding: var(--space-sm) var(--space-md);
  background: color-mix(in srgb, var(--error-color) 8%, transparent);
  border-radius: var(--radius-sm);
  margin: 0 auto var(--space-sm);
  max-width: 360px;
}
.vision-camera-cta__primary :deep(.n-button__content) {
  font-size: var(--font-md);
}
.vision-camera-cta__fallback {
  margin-top: calc(-1 * var(--space-xs));
}
.vision-camera-cta__preview {
  margin-top: var(--space-md);
  padding-top: var(--space-lg);
  border-top: 1px dashed var(--border-color);
}
.vision-camera-cta__preview-img {
  max-width: 200px;
  max-height: 160px;
  object-fit: contain;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-color);
  margin: 0 auto var(--space-md);
  display: block;
}
.vision-camera-cta__preview-actions {
  display: flex;
  gap: var(--space-sm);
  justify-content: center;
}

/* 超窄屏（iPhone SE 等 ≤400px）：取景框四角和快门按钮略缩小 */
@media (--phone) {
  .frame-corner {
    width: 20px;
    height: 20px;
  }
  .vision-camera-cta {
    padding: var(--space-xl) var(--space-lg);
  }
  .vision-camera-cta__icon {
    /* stylelint-disable-next-line declaration-property-value-allowed-list -- 装饰性 emoji 图标：保留窄屏缩小档位 */
    font-size: 2.5rem;
  }
}
</style>
