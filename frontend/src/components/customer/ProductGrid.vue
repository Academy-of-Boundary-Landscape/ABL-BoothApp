<template>
  <draggable
    v-model="localList"
    class="product-grid"
    :class="[`card-size-${cardSize}`, { 'is-editing': editable }]"
    :style="{ '--pg-media-pad': mediaPadPercent }"
    item-key="id"
    :animation="250"
    ghost-class="ghost-card"
    drag-class="drag-card"
    :disabled="!editable"
    :force-fallback="true"
    :fallback-on-body="false"
    :fallback-tolerance="3"
    :touch-start-threshold="4"
    @end="handleDragEnd"
  >
    <template #item="{ element: product }">
      <n-card
        class="product-card"
        :class="{
          'out-of-stock': product.onsite_qty === 0,
          'low-stock': !editable && product.onsite_qty > 0 && product.onsite_qty <= 10,
          'just-added': animatingIds.has(product.id),
        }"
        embedded
        :content-style="{ padding: 0 }"
        :bordered="false"
      >
        <div class="card-inner" @click="handleCardClick(product)">
          <div class="media-box">
            <n-image
              v-if="product.image_url"
              class="media-img"
              :src="product.image_url"
              :alt="product.name"
              preview-disabled
              :img-props="{ loading: 'lazy', draggable: false }"
            >
              <!-- ✅ 加载中：Skeleton -->
              <template #placeholder>
                <div class="media-skeleton">
                  <n-skeleton class="sk-img" :sharp="false" height="100%" width="100%" />
                  <div class="sk-shine" />
                </div>
              </template>

              <!-- ✅ 加载失败：Skeleton + 提示 -->
              <template #error>
                <div class="media-error">
                  <n-skeleton class="sk-img" :sharp="false" height="100%" width="100%" />
                  <div class="err-text">图片加载失败</div>
                </div>
              </template>
            </n-image>

            <div v-else class="media-placeholder">
              <span class="placeholder-emoji">{{ product.name?.charAt(0) || '🛍️' }}</span>
            </div>

            <div v-if="editable" class="edit-overlay">
              <span class="drag-icon">✋ 拖动排序</span>
            </div>

            <template v-else>
              <!-- 低库存：角标 + 底部库存条 -->
              <template v-if="product.onsite_qty > 0 && product.onsite_qty <= 10">
                <div class="chip stock-warning">
                  <span>仅剩 {{ product.onsite_qty }} 件</span>
                </div>
                <div class="stock-bar">
                  <div
                    class="stock-bar-fill"
                    :class="{ critical: product.onsite_qty <= 3 }"
                    :style="{
                      width: product.stocked_qty
                        ? Math.min((product.onsite_qty / product.stocked_qty) * 100, 100) + '%'
                        : '0%',
                    }"
                  ></div>
                </div>
              </template>

              <!-- 售罄 -->
              <div v-if="product.onsite_qty === 0" class="sold-overlay">
                <div class="sold-badge">已售罄</div>
              </div>
            </template>
          </div>

          <div class="info-box">
            <div class="title" :title="product.name">
              {{ product.name }}
            </div>

            <div class="bottom-row">
              <div class="price-wrapper">
                <Money :value="product.unit_price" size="lg" />
              </div>

              <div class="action-icon" v-if="!editable && product.onsite_qty > 0"></div>
            </div>
          </div>
        </div>
      </n-card>
    </template>
  </draggable>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import draggable from 'vuedraggable'
import { NCard, NImage, NSkeleton } from 'naive-ui'
import { useThemeStore } from '@/stores/themeStore'
import { Money } from '@/components/ui'
import type { Schemas } from '@/api/client'

const themeStore = useThemeStore()

// 根据全局偏好计算图片区 padding-top（= 高/宽 × 100%）
// 3:4 竖版 → 4/3 = 133.3%；1:1 方形 → 100%
const mediaPadPercent = computed(() => {
  return themeStore.productImageAspect === '1:1' ? '100%' : '133.33%'
})

const props = withDefaults(
  defineProps<{
    products?: Schemas['ProductEventProduct'][]
    cardSize?: 'small' | 'medium' | 'large'
    editable?: boolean
  }>(),
  {
    products: () => [],
    cardSize: 'medium',
    editable: false,
  }
)

const emit = defineEmits<{
  (e: 'addToCart', product: Schemas['ProductEventProduct']): void
  (e: 'update:products', products: Schemas['ProductEventProduct'][]): void
  (e: 'order-changed'): void
}>()

const localList = ref<Schemas['ProductEventProduct'][]>([])
const animatingIds = ref(new Set<number>())

watch(
  () => props.products,
  (val) => {
    if (!props.editable) localList.value = Array.isArray(val) ? [...val] : []
    if (props.editable && localList.value.length === 0)
      localList.value = Array.isArray(val) ? [...val] : []
  },
  { immediate: true }
)

function handleCardClick(product: Schemas['ProductEventProduct']) {
  if (props.editable) return
  if (product?.onsite_qty <= 0) return
  // Trigger add-to-cart animation
  animatingIds.value = new Set([...animatingIds.value, product.id])
  setTimeout(() => {
    const next = new Set(animatingIds.value)
    next.delete(product.id)
    animatingIds.value = next
  }, 400)
  emit('addToCart', product)
}

function handleDragEnd() {
  if (!props.editable) return
  const next = [...localList.value]
  emit('update:products', next)
  emit('order-changed')
}
</script>

<style scoped>
.product-grid {
  --pg-bg: var(--card-bg-color);
  --pg-border: var(--border-color);
  --pg-accent: var(--accent-color);
  --pg-media-pad: 133%;

  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--min-col), 1fr));
  gap: var(--space-md);
  padding: var(--space-xs);
  align-content: start;
}

.product-grid.card-size-small {
  --min-col: 110px;
}
.product-grid.card-size-medium {
  --min-col: 150px;
}
.product-grid.card-size-large {
  --min-col: 220px;
}

/* 小号卡片：缩字、缩按钮、缩内边距 —— 否则在 3:4 + 110px 宽时
   bottom-row 的 ¥15.00 会被加号按钮挤到省略号 (15...) */
.card-size-small .info-box {
  padding: var(--space-sm) var(--space-sm);
  gap: var(--space-xs);
}
.card-size-small .title {
  font-size: var(--font-sm);
  line-height: 1.25;
  font-weight: var(--weight-bold);
}
.card-size-small .bottom-row {
  gap: var(--space-sm);
}
.card-size-small .price-wrapper :deep(.money.lg) {
  font-size: var(--font-base);
}
.card-size-small .action-icon {
  width: 36px;
  height: 36px;
}
.card-size-small .action-icon::before {
  width: 14px;
  height: 2px;
}
.card-size-small .action-icon::after {
  width: 2px;
  height: 14px;
}

.product-card {
  border-radius: var(--radius-lg);
  transition:
    transform 0.2s,
    box-shadow 0.2s;
  border: 1px solid var(--pg-border);
  background-color: var(--pg-bg);
  overflow: hidden;
  height: 100%;
}

.product-grid:not(.is-editing) .product-card:hover {
  transform: translateY(-3px);
  box-shadow: var(--shadow-md);
}

.card-inner {
  height: 100%;
  display: flex;
  flex-direction: column;
  cursor: pointer;
  user-select: none;
}
.product-card.out-of-stock .card-inner {
  cursor: default;
  opacity: 0.6;
}

.media-box {
  position: relative;
  width: 100%;
  background-color: var(--bg-secondary);
  overflow: hidden;
}

/* 图片区域宽高比：由 themeStore.productImageAspect 通过 inline style 传入。
   3:4（默认）=> 133.33%（竖向，适合立绘/明信片/海报）
   1:1        => 100%（方形，适合亚克力/徽章/周边小物）
   如果 inline style 未提供，兜底 133% 保持旧行为。 */
.media-box::before {
  content: '';
  display: block;
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 商品图区宽高比占位（百分比 padding-top），属内容几何，不是间距刻度 */
  padding-top: var(--pg-media-pad);
}

:deep(.media-img),
.media-placeholder {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

/* ✅ 直接命中 n-image 根节点，保证居中布局生效 */
:deep(.media-img) {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ✅ Naive 内部 wrapper 拉满并居中 */
:deep(.media-img .n-image-wrapper) {
  width: 100% !important;
  height: 100% !important;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 图片居中完整显示（宁可留白，不拉伸） */
:deep(.media-img img),
:deep(.media-img .n-image-img) {
  width: auto;
  height: auto;
  max-width: 100%;
  max-height: 100%;
  display: block;
  object-fit: contain;
  object-position: center center;
  background: var(--bg-secondary);
}

/* Skeleton / error 覆盖整个 media 区域 */
.media-skeleton,
.media-error {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.media-skeleton .sk-img,
.media-error .sk-img {
  width: 100%;
  height: 100%;
}

.media-skeleton {
  overflow: hidden;
}
.media-skeleton .sk-shine {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    110deg,
    transparent 0%,
    color-mix(in srgb, var(--text-white) 20%, transparent) 30%,
    transparent 60%
  );
  transform: translateX(-60%);
  animation: shine 1.2s infinite;
}

@keyframes shine {
  0% {
    transform: translateX(-60%);
  }
  100% {
    transform: translateX(60%);
  }
}

.media-error {
  display: flex;
  align-items: center;
  justify-content: center;
}
.media-error .err-text {
  position: absolute;
  bottom: 8px;
  left: 8px;
  right: 8px;
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  color: var(--text-muted);
  background: color-mix(in srgb, var(--card-bg-color) 85%, transparent);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-sm);
  text-align: center;
}

.media-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-2xl);
  opacity: 0.5;
}

.chip {
  position: absolute;
  top: 6px;
  right: 6px;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-md);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  color: var(--text-white);
  background: color-mix(in srgb, var(--tooltip-bg) 55%, transparent);
  backdrop-filter: blur(6px);
}
.chip.stock-warning {
  background: var(--warning-color);
  animation: stock-pulse 2s ease-in-out infinite;
}
.product-card.low-stock {
  border-color: var(--warning-color);
}

@keyframes stock-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

/* 库存进度条：贴在图片区域底部 */
.stock-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: color-mix(in srgb, var(--overlay-color) 25%, transparent);
}
.stock-bar-fill {
  height: 100%;
  background: var(--warning-color);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  transition: width 0.3s;
}
.stock-bar-fill.critical {
  background: var(--error-color);
}

/* 售罄：磨砂 + 标签，自适应明暗主题 */
.sold-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--card-bg-color) 70%, transparent);
  backdrop-filter: blur(6px);
}

.sold-badge {
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-pill);
  font-weight: var(--weight-bold);
  letter-spacing: 0.06em;
  font-size: var(--font-xs);
  color: var(--text-white);
  background: var(--tooltip-bg);
  box-shadow: var(--shadow-lg);
  transform: rotate(-6deg);
}

/* 信息区 */
.info-box {
  padding: var(--space-sm) var(--space-sm);
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--space-sm);
  min-width: 0;
}

.title {
  font-size: var(--font-base);
  line-height: 1.35;
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.bottom-row {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  /* 价格与「+」一行放不下时让「+」下移，而不是把价格截断。 */
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.price-wrapper {
  color: var(--pg-accent);
  line-height: 1;
  /* 价格永不截断：不缩、不省略，放不下就把按钮挤到下一行。 */
  flex-shrink: 0;
  white-space: nowrap;
}

.action-icon {
  flex-shrink: 0;
  /* 换行到独立一行时靠右对齐（space-between 对单元素不生效）。 */
  margin-left: auto;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  /* 描边样式：一屏十几张卡，实心红圆太重；整张卡本来就可点，「+」只是提示 */
  border: 2px solid var(--accent-color);
  background: var(--card-bg-color);
  transition:
    transform 0.15s,
    background-color 0.15s;
  position: relative;
}
/* 用伪元素画十字，确保像素级居中 */
.action-icon::before,
.action-icon::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  background: var(--accent-color);
  border-radius: var(--radius-sm);
  transform: translate(-50%, -50%);
}
.action-icon::before {
  width: 18px;
  height: 3px;
}
.action-icon::after {
  width: 3px;
  height: 18px;
}
.action-icon:active {
  transform: scale(0.88);
  background: var(--accent-color-light);
}

/* 拖拽视觉 */
.ghost-card {
  opacity: 0.5;
  background: var(--bg-secondary);
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-lg);
}
.drag-card {
  opacity: 1;
  transform: scale(1.05) rotate(2deg);
  box-shadow: var(--shadow-xl);
  z-index: 1000;
  cursor: grabbing;
}

.is-editing .product-card {
  cursor: grab;
  animation: shake 2s infinite ease-in-out;
}
.is-editing .product-card:active {
  cursor: grabbing;
}

.edit-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--overlay-color) 12%, transparent);
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px dashed var(--pg-accent);
}
.drag-icon {
  background: var(--pg-accent);
  color: var(--text-white);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-xl);
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  box-shadow: var(--shadow-sm);
}

@keyframes shake {
  0% {
    transform: rotate(0deg);
  }
  25% {
    transform: rotate(0.5deg);
  }
  75% {
    transform: rotate(-0.5deg);
  }
  100% {
    transform: rotate(0deg);
  }
}

.product-card.just-added {
  animation: add-pulse 0.4s ease;
}

@keyframes add-pulse {
  0% {
    transform: scale(1);
  }
  30% {
    transform: scale(0.93);
    /* stylelint-disable-next-line declaration-property-value-allowed-list -- 新增商品强调脉冲环（0 0 0 3px），非设计阴影档 */
    box-shadow: 0 0 0 3px var(--accent-color);
  }
  60% {
    transform: scale(1.03);
  }
  100% {
    transform: scale(1);
    box-shadow: none;
  }
}

@media (--phone) {
  /* small 卡片的「+」手机上门禁要求 ≥44px；桌面 36px 更紧凑可以保留。 */
  .card-size-small .action-icon {
    width: 44px;
    height: 44px;
  }
}
</style>
