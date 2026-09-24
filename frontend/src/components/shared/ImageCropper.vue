<template>
  <!-- ui-boundary-ignore: 全屏裁剪交互依赖 esc/transform-origin 与自定义遮罩，AppModal 无法承载 -->
  <n-modal
    :show="show"
    :mask-closable="false"
    :close-on-esc="true"
    transform-origin="center"
    @esc="handleClose"
    @update:show="(v) => !v && handleClose()"
  >
    <div class="cropper-root">
      <!-- 顶部标题栏 -->
      <header class="cropper-header">
        <div class="cropper-title">
          裁剪图片
          <span v-if="batchLabel" class="cropper-batch-label">{{ batchLabel }}</span>
        </div>
        <button class="cropper-close" @click="handleClose" aria-label="关闭">×</button>
      </header>

      <!-- 裁剪舞台 -->
      <div class="cropper-stage" ref="stageRef">
        <div v-if="!loaded" class="cropper-loading">
          <n-spin size="small" />
          <span>加载图片...</span>
        </div>

        <div class="cropper-canvas-wrap" v-show="loaded">
          <img
            ref="imgRef"
            :src="imgSrc"
            class="cropper-image"
            alt="裁剪图"
            draggable="false"
            @load="onImageLoad"
          />

          <!-- 遮罩四块 -->
          <template v-if="loaded">
            <div class="mask mask-top" :style="maskTopStyle"></div>
            <div class="mask mask-bottom" :style="maskBottomStyle"></div>
            <div class="mask mask-left" :style="maskLeftStyle"></div>
            <div class="mask mask-right" :style="maskRightStyle"></div>

            <!-- 裁剪框 -->
            <div class="crop-box" :style="cropBoxStyle" @pointerdown="startMove">
              <!-- 3x3 辅助网格线 -->
              <div class="crop-grid">
                <span class="grid-line v1"></span>
                <span class="grid-line v2"></span>
                <span class="grid-line h1"></span>
                <span class="grid-line h2"></span>
              </div>
              <!-- 四角拖拽手柄 -->
              <span
                v-for="corner in ['nw', 'ne', 'sw', 'se']"
                :key="corner"
                :class="['handle', `handle-${corner}`]"
                @pointerdown.stop="startResize(corner, $event)"
              ></span>
            </div>
          </template>
        </div>
      </div>

      <!-- 比例切换 -->
      <div class="cropper-ratios">
        <button
          v-for="r in ratios"
          :key="r.key"
          :class="['ratio-btn', { active: aspect === r.key }]"
          @click="setAspect(r.key)"
        >
          {{ r.label }}
        </button>
      </div>

      <!-- 底部按钮 -->
      <footer class="cropper-footer">
        <n-button @click="handleSkip">跳过（使用原图）</n-button>
        <n-button tertiary @click="resetBox">重置</n-button>
        <n-button
          type="primary"
          :loading="confirming"
          :disabled="confirming || !loaded"
          @click="confirmCrop"
        >
          {{ confirming ? '处理中...' : '确定裁剪' }}
        </n-button>
      </footer>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
// ui-boundary-ignore: 全屏裁剪交互保留裸 n-modal，理由见模板注释
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { NButton, NModal, NSpin } from 'naive-ui'

interface Props {
  show?: boolean
  file?: File | null
  defaultAspect?: string // 'free' | '1:1' | '3:4'
  batchLabel?: string // e.g. "2 / 5" for multi-file batch
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  file: null,
  defaultAspect: 'free',
  batchLabel: '',
})

const emit = defineEmits<{
  (e: 'confirm', file: File): void
  (e: 'skip', file: File | null): void
  (e: 'close'): void
}>()

const ratios = [
  { key: 'free', label: '自由', ratio: null },
  { key: '1:1', label: '1 : 1', ratio: 1 },
  { key: '3:4', label: '3 : 4', ratio: 3 / 4 },
]

const imgSrc = ref('')
const loaded = ref(false)
const confirming = ref(false)
const aspect = ref(props.defaultAspect)

const stageRef = ref<HTMLDivElement | null>(null)
const imgRef = ref<HTMLImageElement | null>(null)

interface Box {
  x: number
  y: number
  w: number
  h: number
}

// 裁剪框用百分比表示（相对于图片自然尺寸），[0,1]
// x, y = 左上角；w, h = 宽高
const cropPct = ref<Box>({ x: 0.1, y: 0.1, w: 0.8, h: 0.8 })

// 图片自然尺寸和渲染尺寸（随容器变化）
const imgNatural = ref({ w: 0, h: 0 })
const imgRendered = ref({ w: 0, h: 0 })

// ===== 监听 props.file 变化 → 加载 URL =====
let currentObjectUrl: string | null = null
watch(
  () => props.file,
  (newFile) => {
    loaded.value = false
    if (currentObjectUrl) {
      URL.revokeObjectURL(currentObjectUrl)
      currentObjectUrl = null
    }
    if (newFile) {
      currentObjectUrl = URL.createObjectURL(newFile)
      imgSrc.value = currentObjectUrl
    } else {
      imgSrc.value = ''
    }
  },
  { immediate: true }
)

watch(
  () => props.show,
  (v) => {
    if (v) {
      // 打开时重置比例和裁剪框。
      // 若图片已加载（NModal 在某些 display-directive 下不会 unmount 图片，或重复使用同一 File），
      // onImageLoad 不会再次触发，这里直接兜底一次 initCropBox，保证每次打开都回到初始状态。
      aspect.value = props.defaultAspect
      if (loaded.value && imgNatural.value.w) {
        initCropBox()
      }
    }
  }
)

onBeforeUnmount(() => {
  if (currentObjectUrl) URL.revokeObjectURL(currentObjectUrl)
  stopPointerListeners()
  if (resizeObserver) resizeObserver.disconnect()
})

// ===== 图片加载完 =====
let resizeObserver: ResizeObserver | null = null

function onImageLoad() {
  const img = imgRef.value
  if (!img) return
  imgNatural.value = { w: img.naturalWidth, h: img.naturalHeight }
  // 等下一帧再测量渲染尺寸（浏览器完成布局）
  requestAnimationFrame(() => {
    const rect = img.getBoundingClientRect()
    imgRendered.value = { w: rect.width, h: rect.height }
    initCropBox()
    loaded.value = true
  })

  // 清掉可能的旧 observer 再建新的
  if (resizeObserver) {
    resizeObserver.disconnect()
  }
  resizeObserver = new ResizeObserver(() => {
    if (!imgRef.value) return
    const r = imgRef.value.getBoundingClientRect()
    imgRendered.value = { w: r.width, h: r.height }
  })
  resizeObserver.observe(img)
}

// ===== 初始化裁剪框（默认在图片中央，占 80%）=====
function initCropBox() {
  if (aspect.value === 'free') {
    cropPct.value = { x: 0.1, y: 0.1, w: 0.8, h: 0.8 }
  } else {
    const ratioMap: Record<string, number> = { '1:1': 1, '3:4': 3 / 4 }
    fitBoxToAspect(ratioMap[aspect.value])
  }
}

function fitBoxToAspect(targetRatio: number) {
  // 以当前图片宽高比为参考，使裁剪框居中且最大化
  const imgRatio = imgNatural.value.w / imgNatural.value.h
  let w: number, h: number
  if (targetRatio >= imgRatio) {
    // 目标比图片宽 → 以宽为限
    w = 0.9
    h = (w * imgRatio) / targetRatio
  } else {
    // 目标比图片窄 → 以高为限
    h = 0.9
    w = (h * targetRatio) / imgRatio
  }
  cropPct.value = {
    x: (1 - w) / 2,
    y: (1 - h) / 2,
    w,
    h,
  }
}

// ===== 样式计算 =====
const cropBoxStyle = computed(() => ({
  left: cropPct.value.x * 100 + '%',
  top: cropPct.value.y * 100 + '%',
  width: cropPct.value.w * 100 + '%',
  height: cropPct.value.h * 100 + '%',
}))

const maskTopStyle = computed(() => ({
  top: 0,
  left: 0,
  right: 0,
  height: cropPct.value.y * 100 + '%',
}))
const maskBottomStyle = computed(() => ({
  bottom: 0,
  left: 0,
  right: 0,
  top: (cropPct.value.y + cropPct.value.h) * 100 + '%',
}))
const maskLeftStyle = computed(() => ({
  top: cropPct.value.y * 100 + '%',
  height: cropPct.value.h * 100 + '%',
  left: 0,
  width: cropPct.value.x * 100 + '%',
}))
const maskRightStyle = computed(() => ({
  top: cropPct.value.y * 100 + '%',
  height: cropPct.value.h * 100 + '%',
  right: 0,
  left: (cropPct.value.x + cropPct.value.w) * 100 + '%',
}))

// ===== 拖拽逻辑 =====
type DragState =
  | {
      type: 'move'
      startClient: { x: number; y: number }
      startBox: Box
      pointerId: number
    }
  | {
      type: 'resize'
      corner: string
      startClient: { x: number; y: number }
      startBox: Box
      pointerId: number
    }

let dragState: DragState | null = null

function startMove(e: PointerEvent) {
  if (!loaded.value) return
  e.preventDefault()
  dragState = {
    type: 'move',
    startClient: { x: e.clientX, y: e.clientY },
    startBox: { ...cropPct.value },
    pointerId: e.pointerId,
  }
  startPointerListeners()
}

function startResize(corner: string, e: PointerEvent) {
  if (!loaded.value) return
  e.preventDefault()
  dragState = {
    type: 'resize',
    corner,
    startClient: { x: e.clientX, y: e.clientY },
    startBox: { ...cropPct.value },
    pointerId: e.pointerId,
  }
  startPointerListeners()
}

function onPointerMove(e: PointerEvent) {
  if (!dragState) return
  if (e.pointerId !== dragState.pointerId) return

  const { w: renderW, h: renderH } = imgRendered.value
  if (!renderW || !renderH) return

  const dx = (e.clientX - dragState.startClient.x) / renderW
  const dy = (e.clientY - dragState.startClient.y) / renderH

  if (dragState.type === 'move') {
    const box = { ...dragState.startBox }
    box.x = clamp(box.x + dx, 0, 1 - box.w)
    box.y = clamp(box.y + dy, 0, 1 - box.h)
    cropPct.value = box
  } else if (dragState.type === 'resize') {
    cropPct.value = resizeBox(dragState.startBox, dragState.corner, dx, dy)
  }
}

function onPointerUp(e: PointerEvent) {
  if (dragState && dragState.pointerId === e.pointerId) {
    dragState = null
    stopPointerListeners()
  }
}

function startPointerListeners() {
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerUp)
}
function stopPointerListeners() {
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  window.removeEventListener('pointercancel', onPointerUp)
}

// 角点拖拽：根据 corner 计算新的裁剪框
// dx/dy 是百分比增量（基于渲染尺寸）
function resizeBox(start: Box, corner: string, dx: number, dy: number) {
  const imgRatio = imgNatural.value.w / imgNatural.value.h
  const ratioMap: Record<string, number> = { '1:1': 1, '3:4': 3 / 4 }
  const lockRatio = aspect.value === 'free' ? null : ratioMap[aspect.value]

  // 起始边界
  let left = start.x
  let top = start.y
  let right = start.x + start.w
  let bottom = start.y + start.h

  // 根据 corner 推算对角固定点 (anchor)
  const anchorX = corner.includes('w') ? right : left
  const anchorY = corner.includes('n') ? bottom : top

  // 根据 corner 哪个点在动
  if (corner === 'nw') {
    left = start.x + dx
    top = start.y + dy
  } else if (corner === 'ne') {
    right = start.x + start.w + dx
    top = start.y + dy
  } else if (corner === 'sw') {
    left = start.x + dx
    bottom = start.y + start.h + dy
  } else if (corner === 'se') {
    right = start.x + start.w + dx
    bottom = start.y + start.h + dy
  }

  // 最小尺寸限制
  const minPct = 0.05

  // 计算初步宽高
  let w = Math.max(minPct, right - left)
  let h = Math.max(minPct, bottom - top)

  // 如果锁定比例，调整 w 或 h 使满足 w/h 在图片空间里 = lockRatio
  // 裁剪后比例 = (w * imgNW) / (h * imgNH) = (w/h) * imgNW/imgNH
  // 要 = lockRatio → w/h = lockRatio / (imgNW/imgNH) = lockRatio / imgRatio
  if (lockRatio !== null) {
    const pctRatio = lockRatio / imgRatio
    // 以较大者为基准
    if (w / h > pctRatio) {
      // w 太大，按 h 定 w
      w = h * pctRatio
    } else {
      h = w / pctRatio
    }
  }

  // 根据锚点重算 left/top/right/bottom
  let newX, newY
  if (corner.includes('w')) {
    newX = anchorX - w
  } else {
    newX = anchorX
  }
  if (corner.includes('n')) {
    newY = anchorY - h
  } else {
    newY = anchorY
  }

  // 边界钳制
  newX = clamp(newX, 0, 1 - w)
  newY = clamp(newY, 0, 1 - h)
  // 宽高钳制（图片外不能超）
  w = Math.min(w, 1 - newX)
  h = Math.min(h, 1 - newY)

  return { x: newX, y: newY, w, h }
}

function clamp(v: number, min: number, max: number) {
  return Math.max(min, Math.min(max, v))
}

// ===== 比例切换 =====
function setAspect(key: string) {
  aspect.value = key
  if (key === 'free') return
  const ratioMap: Record<string, number> = { '1:1': 1, '3:4': 3 / 4 }
  fitBoxToAspect(ratioMap[key])
}

// ===== 重置 =====
function resetBox() {
  initCropBox()
}

// ===== 确认裁剪 =====
async function confirmCrop() {
  const img = imgRef.value
  const file = props.file
  if (!file || !img || confirming.value) return
  confirming.value = true
  try {
    const naturalW = imgNatural.value.w
    const naturalH = imgNatural.value.h
    const sx = Math.round(cropPct.value.x * naturalW)
    const sy = Math.round(cropPct.value.y * naturalH)
    const sw = Math.max(1, Math.round(cropPct.value.w * naturalW))
    const sh = Math.max(1, Math.round(cropPct.value.h * naturalH))

    const canvas = document.createElement('canvas')
    canvas.width = sw
    canvas.height = sh
    const ctx = canvas.getContext('2d')
    ctx!.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh)

    // 输出类型和原文件一致；JPEG 默认 0.92 质量
    const mime = file.type || 'image/jpeg'
    const quality = mime.includes('png') ? undefined : 0.92

    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, mime, quality))
    if (!blob) throw new Error('toBlob returned null')

    const croppedFile = new File([blob], file.name, {
      type: mime,
      lastModified: Date.now(),
    })
    emit('confirm', croppedFile)
  } catch (err) {
    console.error('[ImageCropper] confirm failed:', err)
  } finally {
    confirming.value = false
  }
}

function handleSkip() {
  emit('skip', props.file)
}

function handleClose() {
  emit('close')
}
</script>

<style scoped>
.cropper-root {
  width: min(92vw, 900px);
  height: 90vh;
  max-height: 900px;
  background: var(--card-bg-color);
  border-radius: var(--radius-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* 顶部 */
.cropper-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-md) var(--space-lg);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}
.cropper-title {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
}
.cropper-batch-label {
  font-size: var(--font-sm);
  font-weight: var(--weight-medium);
  color: var(--text-muted);
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-secondary);
  border-radius: var(--radius-pill);
}
.cropper-close {
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: var(--font-xl);
  line-height: 1;
  cursor: pointer;
  border-radius: 50%;
  transition: background-color 0.15s;
}
.cropper-close:hover {
  background: var(--bg-secondary);
  color: var(--primary-text-color);
}

/* 裁剪舞台 */
.cropper-stage {
  flex: 1;
  min-height: 0;
  /* stylelint-disable-next-line color-no-hex -- 裁剪舞台需纯黑背景衬托图片与遮罩边界，色板无纯黑 token */
  background: #000;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  user-select: none;
}
.cropper-loading {
  color: var(--text-white);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  font-size: var(--font-sm);
}
.cropper-canvas-wrap {
  position: relative;
  max-width: 100%;
  max-height: 100%;
  display: inline-block;
  line-height: 0; /* 消除 img 下方的基线间隙 */
}
.cropper-image {
  display: block;
  max-width: 100%;
  max-height: calc(90vh - 180px);
  width: auto;
  height: auto;
  user-select: none;
  -webkit-user-drag: none;
}

/* 遮罩 */
.mask {
  position: absolute;
  background: var(--overlay-color);
  pointer-events: none;
}

/* 裁剪框 */
.crop-box {
  position: absolute;
  border: 2px solid var(--text-white);
  box-sizing: border-box;
  cursor: move;
  touch-action: none;
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 裁剪框 1px 硬描边贴合图片边缘，软阴影无法替代 */
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--overlay-color) 75%, transparent);
}

/* 3x3 辅助网格 */
.crop-grid {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.grid-line {
  position: absolute;
  background: color-mix(in srgb, var(--text-white) 40%, transparent);
}
.grid-line.v1,
.grid-line.v2 {
  top: 0;
  bottom: 0;
  width: 1px;
}
.grid-line.v1 {
  left: 33.33%;
}
.grid-line.v2 {
  left: 66.67%;
}
.grid-line.h1,
.grid-line.h2 {
  left: 0;
  right: 0;
  height: 1px;
}
.grid-line.h1 {
  top: 33.33%;
}
.grid-line.h2 {
  top: 66.67%;
}

/* 四角手柄 */
.handle {
  position: absolute;
  width: 16px;
  height: 16px;
  background: var(--text-white);
  border: 2px solid var(--accent-color);
  border-radius: var(--radius-sm);
  touch-action: none;
}
/* 触控目标扩大区（不可见） */
.handle::before {
  content: '';
  position: absolute;
  inset: -14px;
}
.handle-nw {
  top: -10px;
  left: -10px;
  cursor: nwse-resize;
}
.handle-ne {
  top: -10px;
  right: -10px;
  cursor: nesw-resize;
}
.handle-sw {
  bottom: -10px;
  left: -10px;
  cursor: nesw-resize;
}
.handle-se {
  bottom: -10px;
  right: -10px;
  cursor: nwse-resize;
}

/* 比例切换 */
.cropper-ratios {
  display: flex;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  border-top: 1px solid var(--border-color);
  justify-content: center;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.ratio-btn {
  padding: var(--space-sm) var(--space-lg);
  background: var(--card-bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-pill);
  color: var(--text-muted);
  font-size: var(--font-sm);
  cursor: pointer;
  transition: all 0.15s;
  user-select: none;
}
.ratio-btn:hover {
  border-color: var(--accent-color);
  color: var(--accent-color);
}
.ratio-btn.active {
  background: var(--accent-color);
  border-color: var(--accent-color);
  color: var(--text-white);
  font-weight: var(--weight-bold);
}

/* 底部按钮 */
.cropper-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-lg);
  border-top: 1px solid var(--border-color);
  flex-shrink: 0;
}

/* 移动端适配 */
@media (--phone) {
  .cropper-root {
    width: 100vw;
    height: 100vh;
    max-width: none;
    max-height: none;
    border-radius: 0;
  }
  .cropper-header {
    padding: var(--space-sm) var(--space-md);
  }
  .cropper-ratios {
    padding: var(--space-sm);
    gap: var(--space-sm);
  }
  .ratio-btn {
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-xs);
  }
  .cropper-footer {
    padding: var(--space-sm) var(--space-md);
    gap: var(--space-sm);
  }
  .cropper-footer :deep(.n-button) {
    flex: 1;
  }
  .handle {
    width: 20px;
    height: 20px;
  }
  .handle-nw {
    top: -12px;
    left: -12px;
  }
  .handle-ne {
    top: -12px;
    right: -12px;
  }
  .handle-sw {
    bottom: -12px;
    left: -12px;
  }
  .handle-se {
    bottom: -12px;
    right: -12px;
  }
}
</style>
