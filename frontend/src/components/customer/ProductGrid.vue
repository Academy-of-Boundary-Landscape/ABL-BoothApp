<template>
  <draggable
  v-model="localList"
  class="product-grid"
  :class="[`card-size-${cardSize}`, { 'is-editing': editable }]"
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
        :class="{ 'out-of-stock': product.current_stock === 0 }"
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
            />
            <div v-else class="media-placeholder">
              <span class="placeholder-emoji">{{ product.name?.charAt(0) || '🛍️' }}</span>
            </div>

            <div v-if="editable" class="edit-overlay">
              <span class="drag-icon">✋ 拖动排序</span>
            </div>

            <template v-else>
              <div v-if="product.current_stock > 0 && product.current_stock <= 10" class="chip stock-warning">
                <span>剩 {{ product.current_stock }}</span>
              </div>
              <div v-if="product.current_stock === 0" class="sold-overlay">
                <span class="sold-text">SOLD OUT</span>
              </div>
            </template>
          </div>

          <div class="info-box">
            <div class="title" :title="product.name">
              {{ product.name }}
            </div>

            <div class="bottom-row">
              <div class="price-wrapper">
                <span class="currency">¥</span>
                <span class="value">{{ Number(product.price).toFixed(2) }}</span>
              </div>

              <div class="action-icon" v-if="!editable && product.current_stock > 0">
                <span class="plus-sign">+</span>
              </div>
            </div>
          </div>
        </div>
      </n-card>
    </template>
  </draggable>
</template>

<script setup>
import { ref, watch } from 'vue'
import draggable from 'vuedraggable'
import { NCard, NImage } from 'naive-ui'

const props = defineProps({
  products: { type: Array, default: () => [] },
  cardSize: {
    type: String,
    default: 'medium',
    validator: (v) => ['small', 'medium', 'large'].includes(v)
  },
  editable: { type: Boolean, default: false }
})

const emit = defineEmits(['addToCart', 'update:products', 'order-changed'])

/**
 * ✅ 关键修复：draggable 使用本地数组，避免直接 mutate props.products
 */
const localList = ref([])

watch(
  () => props.products,
  (val) => {
    // 编辑模式下，外部如果重置 products，会打断用户拖拽；通常不建议。
    // 这里选择：只有当“非编辑模式”或“首次进入”时才同步，以防覆盖拖拽过程。
    if (!props.editable) localList.value = Array.isArray(val) ? [...val] : []
    if (props.editable && localList.value.length === 0) localList.value = Array.isArray(val) ? [...val] : []
  },
  { immediate: true }
)

// 点击：编辑模式禁用加购
function handleCardClick(product) {
  if (props.editable) return
  if (product?.current_stock > 0) emit('addToCart', product)
}

// 拖拽结束：把新顺序同步给父组件，并通知保存
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
  --pg-radius: 12px;
  --pg-aspect-ratio: 1 / 1;

  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--min-col), 1fr));
  gap: 12px;
  padding: 4px;
}

.product-grid.card-size-small  { --min-col: 110px; --pg-aspect-ratio: 1/1; }
.product-grid.card-size-medium { --min-col: 150px; --pg-aspect-ratio: 1/1; }
.product-grid.card-size-large  { --min-col: 220px; --pg-aspect-ratio: 4/3; }

.product-card {
  border-radius: var(--pg-radius);
  transition: transform 0.2s, box-shadow 0.2s;
  border: 1px solid var(--pg-border);
  background-color: var(--pg-bg);
  overflow: hidden;
  height: 100%;
}

.product-grid:not(.is-editing) .product-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.08);
}

.card-inner {
  height: 100%;
  display: flex;
  flex-direction: column;
  cursor: pointer;
  user-select: none;
}

.media-box {
  position: relative;
  width: 100%;
  background-color: var(--bg-secondary);
  overflow: hidden;
}

/* 比例：默认 1/1；large 设 4/3 => padding-top = 75% */
.product-grid { --pg-media-pad: 100%; }              /* 1/1 */
.product-grid.card-size-large { --pg-media-pad: 75%; } /* 4/3 = 高/宽 = 3/4 */

.media-box::before {
  content: "";
  display: block;
  padding-top: var(--pg-media-pad);
}
:deep(.media-img),
.media-placeholder {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

/* ✅ Naive n-image 内部 wrapper 也铺满 */
:deep(.media-img .n-image),
:deep(.media-img .n-image .n-image-wrapper),
:deep(.media-img .n-image img) {
  width: 100%;
  height: 100%;
  display: block;
}

:deep(.media-img .n-image img) {
  object-fit: cover;
  object-position: center;
}
.media-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 2.5em;
  opacity: 0.5;
}

.chip {
  position: absolute;
  top: 6px;
  right: 6px;
  padding: 2px 6px;
  border-radius: 6px;
  font-size: 10px;
  font-weight: 800;
  color: white;
  background: rgba(0,0,0,0.6);
  backdrop-filter: blur(4px);
}
.chip.stock-warning { background: #d03050; }

.sold-overlay {
  position: absolute;
  inset: 0;
  background: rgba(255,255,255,0.6);
  display: flex;
  align-items: center;
  justify-content: center;
}
.sold-text {
  background: #333;
  color: #fff;
  padding: 4px 10px;
  font-weight: 900;
  font-size: 12px;
  transform: rotate(-10deg);
}

.info-box {
  padding: 10px 10px;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 6px;
  min-width: 0;
}

.title {
  font-size: 0.92rem;
  line-height: 1.35;
  color: var(--primary-text-color);
  font-weight: 650;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.bottom-row {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  flex-wrap: nowrap;
  gap: 8px;
}

.price-wrapper {
  color: var(--pg-accent);
  line-height: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.currency { font-size: 0.75rem; margin-right: 1px; }
.value { font-size: 1.1rem; font-weight: 900; font-family: sans-serif; }

.action-icon {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--bg-secondary);
  color: var(--primary-text-color);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

/* 拖拽视觉 */
.ghost-card {
  opacity: 0.5;
  background: #e0e0e0;
  border: 2px dashed #999;
  border-radius: var(--pg-radius);
}
.drag-card {
  opacity: 1;
  transform: scale(1.05) rotate(2deg);
  box-shadow: 0 12px 24px rgba(0,0,0,0.2);
  z-index: 1000;
  cursor: grabbing;
}

.is-editing .product-card {
  cursor: grab;
  animation: shake 2s infinite ease-in-out;
}
.is-editing .product-card:active { cursor: grabbing; }

.edit-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px dashed var(--pg-accent);
}
.drag-icon {
  background: var(--pg-accent);
  color: white;
  padding: 4px 10px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 800;
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}

@keyframes shake {
  0% { transform: rotate(0deg); }
  25% { transform: rotate(0.5deg); }
  75% { transform: rotate(-0.5deg); }
  100% { transform: rotate(0deg); }
}
</style>
