<template>
  <div class="barcode-scan">
    <!-- 工具栏：返回列表 + 翻转 / 补光 -->
    <div class="barcode-scan__toolbar">
      <button type="button" class="scan-btn" @click="handleClose">返回商品列表</button>
      <div class="barcode-scan__actions">
        <button type="button" class="scan-btn" @click="flipCamera">翻转</button>
        <button
          v-if="torchSupported"
          type="button"
          class="scan-btn"
          :class="{ 'is-active': torchOn }"
          @click="toggleTorch"
        >
          补光
        </button>
      </div>
    </div>

    <!-- 摄像头不可用 / 引擎加载失败 -->
    <div v-if="cameraError || scannerError" class="barcode-scan__error">
      <p class="barcode-scan__error-text">{{ cameraError || scannerError }}</p>
      <button type="button" class="scan-btn" @click="handleClose">返回商品列表</button>
    </div>

    <div v-else class="barcode-scan__body">
      <div
        ref="viewportRef"
        class="scan-viewport"
        :class="{ 'is-flash-green': flash === 'green', 'is-flash-red': flash === 'red' }"
      >
        <video
          ref="videoRef"
          class="scan-video"
          :class="{ 'scan-video--mirrored': facing === 'user' }"
          autoplay
          playsinline
          muted
        />
        <div class="scan-overlay">
          <div class="scan-frame" :style="frameStyle">
            <span class="scan-frame__corner scan-frame__corner--tl" />
            <span class="scan-frame__corner scan-frame__corner--tr" />
            <span class="scan-frame__corner scan-frame__corner--bl" />
            <span class="scan-frame__corner scan-frame__corner--br" />
          </div>
          <p class="scan-hint" :class="{ 'is-error': hintKind === 'error' }">{{ hintText }}</p>
        </div>
      </div>

      <div class="scan-recent">
        <div class="scan-recent__title">最近扫描</div>
        <ul class="scan-recent__list">
          <li v-for="row in recent" :key="row.key" class="scan-recent__item">{{ row.label }}</li>
          <li v-if="recent.length === 0" class="scan-recent__empty">扫到的商品会出现在这里</li>
        </ul>
      </div>
    </div>

    <!-- 多件命中：唯一允许的弹窗 -->
    <AppModal
      :show="candidates.length > 0"
      title="选择商品"
      size="sm"
      :closable="false"
      @update:show="onCandidatesShow"
    >
      <div class="scan-candidates">
        <button
          v-for="product in candidates"
          :key="product.id"
          type="button"
          class="scan-candidate"
          @click="pickCandidate(product)"
        >
          <span class="scan-candidate__name">{{ product.name }}</span>
          <span class="scan-candidate__code">{{ product.product_code }}</span>
        </button>
      </div>
    </AppModal>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { AppModal } from '@/components/ui'
import { useCamera } from '@/composables/useCamera'
import { useBarcodeScanner, type ScanRoi } from '@/composables/useBarcodeScanner'
import { useScanResultHandler } from '@/composables/useScanResultHandler'
import { matchBarcode } from '@/utils/barcodeMatch'
import { playScanBeep } from '@/utils/scanBeep'
import { formatYuan } from '@/utils/money'
import type { Schemas } from '@/api/client'

const props = withDefaults(
  defineProps<{
    products: Schemas['ProductEventProduct'][]
    /** 购物车当前内容：命中时用于判断同商品是否已被加满库存。 */
    cart?: { id: number; quantity: number }[]
    /** 单次模式：扫到第一个非 ignored 的码就 emit `code` 并停止，不做匹配 / 加购。 */
    single?: boolean
  }>(),
  { single: false, cart: () => [] }
)

const emit = defineEmits<{
  (e: 'add', product: Schemas['ProductEventProduct']): void
  (e: 'code', code: string): void
  (e: 'close'): void
  /** 每处理一个非 ignored 的扫描结果就发一次：父组件据此续期闲置计时器。 */
  (e: 'activity'): void
  /** 多件选择弹窗开关：开着时父组件应停用扫码枪，避免在弹窗背后继续加购。 */
  (e: 'choosing', value: boolean): void
}>()

// ===================== 取景框布局 =====================
const videoRef = ref<HTMLVideoElement | null>(null)
const viewportRef = ref<HTMLDivElement | null>(null)
const viewportSize = ref({ w: 1, h: 1 })

function updateViewportSize() {
  const el = viewportRef.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  viewportSize.value = { w: rect.width || 1, h: rect.height || 1 }
}

/** 横向长条：宽 = 视口宽 80%，高 = 宽的 40%，居中。 */
const frameStyle = computed(() => {
  const width = viewportSize.value.w * 0.8
  return { width: `${width}px`, height: `${width * 0.4}px` }
})

/**
 * 取景框 → 视频像素坐标。视频以 object-fit: cover 铺满视口，先算实际裁切区域，
 * 再把 CSS 像素按同一比例映射到源像素（参考 VisionSearch 的 captureFrame 换算）。
 */
function computeRoi(): ScanRoi {
  const video = videoRef.value
  const viewport = viewportRef.value
  if (!video || !viewport) return { x: 0, y: 0, w: 0, h: 0 }
  const vw = video.videoWidth
  const vh = video.videoHeight
  if (!vw || !vh) return { x: 0, y: 0, w: 0, h: 0 }
  const rect = viewport.getBoundingClientRect()
  const vpW = rect.width
  const vpH = rect.height
  if (!vpW || !vpH) return { x: 0, y: 0, w: 0, h: 0 }

  const videoAspect = vw / vh
  const vpAspect = vpW / vpH
  let srcX = 0
  let srcY = 0
  let srcW = vw
  let srcH = vh
  if (videoAspect > vpAspect) {
    // 视频比视口更宽：左右被裁。
    srcW = vh * vpAspect
    srcX = (vw - srcW) / 2
  } else {
    // 视频比视口更高：上下被裁。
    srcH = vw / vpAspect
    srcY = (vh - srcH) / 2
  }

  const scale = srcW / vpW
  const frameW = vpW * 0.8 * scale
  const frameH = frameW * 0.4
  return {
    x: srcX + (srcW - frameW) / 2,
    y: srcY + (srcH - frameH) / 2,
    w: frameW,
    h: frameH,
  }
}

// ===================== 摄像头 =====================
const {
  stream,
  error: cameraError,
  facing,
  torchSupported,
  torchOn,
  start: startCameraStream,
  stop: stopCameraStream,
  flip: flipCameraStream,
  setTorch,
} = useCamera({ facing: 'environment' })

watch(stream, (s) => {
  if (videoRef.value) videoRef.value.srcObject = s
})
watch(videoRef, (el) => {
  if (el) el.srcObject = stream.value
})

async function flipCamera() {
  await flipCameraStream()
  updateViewportSize()
}

async function toggleTorch() {
  await setTorch(!torchOn.value)
}

// ===================== 扫描器 =====================
const scanner = useBarcodeScanner({
  video: videoRef,
  roi: computeRoi,
  onCode: handleCode,
})
const { error: scannerError } = scanner

const flash = ref<'none' | 'green' | 'red'>('none')
let flashTimer: ReturnType<typeof setTimeout> | null = null
function triggerFlash(kind: 'green' | 'red') {
  flash.value = kind
  clearTimeout(flashTimer ?? undefined)
  flashTimer = setTimeout(() => {
    flash.value = 'none'
  }, 300)
}

const hintText = ref('将条码对准取景框')
const hintKind = ref<'info' | 'error'>('info')
function setHint(message: string, kind: 'info' | 'error' = 'info') {
  hintText.value = message
  hintKind.value = kind
}

interface RecentRow {
  key: number
  label: string
}
const recent = ref<RecentRow[]>([])
let recentKey = 0
function pushRecent(product: Schemas['ProductEventProduct']) {
  const row: RecentRow = {
    key: ++recentKey,
    label: `+1 ${product.name} ${formatYuan(product.unit_price)}`,
  }
  recent.value = [row, ...recent.value].slice(0, 5)
}

const candidates = ref<Schemas['ProductEventProduct'][]>([])
watch(
  () => candidates.value.length,
  (len) => emit('choosing', len > 0)
)

const handler = useScanResultHandler({
  products: () => props.products,
  cart: () => props.cart,
  addToCart: (product) => emit('add', product),
  notify: (message, kind) => {
    // 成功不加提示行（框闪绿 + 提示音 + 最近记录已足够），失败写明原因。
    if (kind === 'error') setHint(message, 'error')
  },
})

function handleAdded(product: Schemas['ProductEventProduct']) {
  triggerFlash('green')
  playScanBeep(true)
  pushRecent(product)
  setHint('将条码对准取景框')
}

function handleCode(code: string) {
  // 单次模式（表单「扫码填入」）：只认码，不匹配、不加购，也不闪框 / 出声。
  if (props.single) {
    if (matchBarcode(code, props.products).kind === 'ignored') return
    emit('activity')
    emit('code', code)
    scanner.pause()
    return
  }

  const outcome = handler.handleCode(code)
  // ignored（价格码等）不算一次有效扫描，不续期闲置计时器。
  if (outcome.kind === 'ignored') return
  emit('activity')
  switch (outcome.kind) {
    case 'added':
      handleAdded(outcome.product)
      break
    case 'sold_out':
    case 'out_of_stock':
    case 'not_found':
      triggerFlash('red')
      playScanBeep(false)
      break
    case 'multiple':
      scanner.pause()
      candidates.value = outcome.products
      break
  }
}

function pickCandidate(product: Schemas['ProductEventProduct']) {
  const outcome = handler.resolveProduct(product)
  if (outcome.kind === 'added') handleAdded(product)
  else if (outcome.kind === 'sold_out' || outcome.kind === 'out_of_stock') {
    triggerFlash('red')
    playScanBeep(false)
  }
  candidates.value = []
  scanner.resume()
}

function onCandidatesShow(show: boolean) {
  if (show) return
  candidates.value = []
  scanner.resume()
}

// ===================== 生命周期 =====================
async function startEverything() {
  const ok = await startCameraStream()
  if (!ok) return
  updateViewportSize()
  await scanner.start()
  // 多件选择弹窗还开着（例如切后台再回来）时不能恢复扫描：弹窗背后扫到码会重复触发。
  if (candidates.value.length > 0) scanner.pause()
}

function handleVisibilityChange() {
  if (document.visibilityState === 'hidden') {
    scanner.stop()
    stopCameraStream()
  } else {
    void startEverything()
  }
}

function handleClose() {
  emit('close')
}

// 转屏 / 布局变化后视口尺寸会变，取景框和 ROI 换算必须跟着更新，
// 否则屏幕上的框和实际解码区域会错位。
let resizeObs: ResizeObserver | null = null

onMounted(() => {
  document.addEventListener('visibilitychange', handleVisibilityChange)
  if (typeof ResizeObserver !== 'undefined') {
    resizeObs = new ResizeObserver(updateViewportSize)
    if (viewportRef.value) resizeObs.observe(viewportRef.value)
  }
  void startEverything()
})

// 取景视口可能在错误态之后才出现，ref 变了要重新 observe。
watch(viewportRef, (el) => {
  if (el && resizeObs) resizeObs.observe(el)
})

onUnmounted(() => {
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  resizeObs?.disconnect()
  resizeObs = null
  clearTimeout(flashTimer ?? undefined)
})
</script>

<style scoped>
.barcode-scan {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  height: 100%;
  min-height: 0;
  /* 与商品列表模式一致：左右 var(--space-md) 页边距，避免工具栏 / 最近扫描贴屏幕边缘。 */
  padding: var(--space-sm) var(--space-md);
  box-sizing: border-box;
}

.barcode-scan__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  flex-shrink: 0;
}

.barcode-scan__actions {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.scan-btn {
  min-height: 40px;
  padding: var(--space-xs) var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-pill);
  background: var(--card-bg-color);
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
  cursor: pointer;
  transition: all 0.15s;
}
.scan-btn.is-active {
  border-color: var(--accent-color);
  background: var(--accent-color);
  color: var(--text-white);
}

.barcode-scan__error {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-lg);
  text-align: center;
}
.barcode-scan__error-text {
  margin: 0;
  color: var(--error-color);
  font-size: var(--font-base);
  line-height: 1.5;
  max-width: var(--page-narrow);
}

.barcode-scan__body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.scan-viewport {
  position: relative;
  flex: 1;
  min-height: 0;
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--overlay-color);
  transition: box-shadow 0.15s;
}
.scan-viewport.is-flash-green {
  outline: 6px solid var(--success-color);
  outline-offset: -6px;
}
.scan-viewport.is-flash-red {
  outline: 6px solid var(--error-color);
  outline-offset: -6px;
}

.scan-video {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

/* 前置摄像头（翻转后）预览做水平镜像，与 VisionSearch 一致。 */
.scan-video--mirrored {
  transform: scaleX(-1);
}

.scan-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.scan-frame {
  position: relative;
  /* 取景框尺寸由 JS 精确给定，不能被 flex 容器压缩，否则屏幕上的框会小于解码 ROI。 */
  flex-shrink: 0;
  border-radius: var(--radius-md);
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 取景框：用超大 spread 阴影实现框外遮罩，非通用阴影 */
  box-shadow: 0 0 0 9999px var(--overlay-color);
}

.scan-frame__corner {
  position: absolute;
  width: 24px;
  height: 24px;
  border-color: var(--accent-color);
  border-style: solid;
}
.scan-frame__corner--tl {
  top: -2px;
  left: -2px;
  border-width: 4px 0 0 4px;
  border-radius: var(--radius-md) 0 0 0;
}
.scan-frame__corner--tr {
  top: -2px;
  right: -2px;
  border-width: 4px 4px 0 0;
  border-radius: 0 var(--radius-md) 0 0;
}
.scan-frame__corner--bl {
  bottom: -2px;
  left: -2px;
  border-width: 0 0 4px 4px;
  border-radius: 0 0 0 var(--radius-md);
}
.scan-frame__corner--br {
  bottom: -2px;
  right: -2px;
  border-width: 0 4px 4px 0;
  border-radius: 0 0 var(--radius-md) 0;
}

/* 提示行绝对定位：不参与 overlay 的垂直居中，取景框才能始终居中。 */
.scan-hint {
  position: absolute;
  left: 0;
  right: 0;
  bottom: var(--space-lg);
  margin: 0;
  color: var(--text-white);
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  text-align: center;
  text-shadow: 0 1px 4px var(--overlay-color);
}
.scan-hint.is-error {
  color: var(--error-color);
}

.scan-recent {
  flex-shrink: 0;
}
.scan-recent__title {
  color: var(--text-muted);
  font-size: var(--font-sm);
  margin-bottom: var(--space-xs);
}
.scan-recent__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}
.scan-recent__item {
  color: var(--secondary-text-color);
  font-size: var(--font-sm);
}
.scan-recent__empty {
  color: var(--text-disabled);
  font-size: var(--font-sm);
}

.scan-candidates {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.scan-candidate {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--card-bg-color);
  cursor: pointer;
  text-align: left;
}
.scan-candidate:hover {
  border-color: var(--accent-color);
}
.scan-candidate__name {
  color: var(--primary-text-color);
  font-size: var(--font-base);
}
.scan-candidate__code {
  color: var(--text-disabled);
  font-size: var(--font-sm);
}
</style>
