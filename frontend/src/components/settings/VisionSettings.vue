<!--
  设置页「AI 视觉识别」区块：顶部是原控制台的 v1.1 推荐体验卡，下面是模型 / 索引面板。
  推荐卡与关闭状态、滚动到面板的逻辑都从旧的 AdminControlPanel.vue 原样搬来，只把样式换成 token。
-->
<template>
  <div id="vision" class="vision-settings">
    <n-space vertical size="large">
      <!-- v1.1 AI 拍照识别 推荐体验 -->
      <section v-if="showAiSpotlight" class="ai-spotlight">
        <button class="ai-spotlight-dismiss" aria-label="关闭" @click="dismissAiSpotlight">
          ×
        </button>
        <div class="ai-spotlight-body">
          <div class="ai-spotlight-emoji">📸</div>
          <div class="ai-spotlight-text">
            <div class="ai-spotlight-badge">可选功能</div>
            <div class="ai-spotlight-title">试试 AI 拍照识别</div>
            <div class="ai-spotlight-desc">
              让顾客拿手机对准商品拍张照，自动识别加入购物车。专为"帮朋友看摊 /
              寄售"场景设计——不用贴条码、不用记 SKU，3 分钟就能跑起来。
            </div>
            <div class="ai-spotlight-actions">
              <n-button type="primary" @click="scrollToVisionPanel"> 🚀 开始配置 </n-button>
              <router-link to="/admin/help" class="ai-spotlight-link"> 先看文档了解 → </router-link>
            </div>
          </div>
        </div>
      </section>

      <!-- AI 视觉识别 -->
      <VisionModelPanel />
    </n-space>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NButton, NSpace } from 'naive-ui'
import { api, unwrap } from '@/api/client'
import VisionModelPanel from '@/components/product/VisionModelPanel.vue'

// ===================== v1.1 AI 推荐体验（原 AdminControlPanel 逻辑）=====================
const AI_SPOTLIGHT_KEY = 'ai_spotlight_dismissed_v1.1'
const aiSpotlightDismissed = ref(localStorage.getItem(AI_SPOTLIGHT_KEY) === '1')
const visionReady = ref(false)

// 仅当用户未手动关闭、且 AI 视觉尚未就绪时显示
const showAiSpotlight = computed(() => !aiSpotlightDismissed.value && !visionReady.value)

function dismissAiSpotlight() {
  aiSpotlightDismissed.value = true
  localStorage.setItem(AI_SPOTLIGHT_KEY, '1')
}

function scrollToVisionPanel() {
  // VisionModelPanel 组件的根元素是 .vision-container
  const el = document.querySelector('.vision-container')
  if (el && typeof el.scrollIntoView === 'function') {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

onMounted(async () => {
  try {
    const res = await unwrap(api.GET('/vision/status'))
    visionReady.value = res?.is_ready === true
  } catch {
    /* 状态拿不到就不隐藏推荐卡，保持原行为 */
  }
})
</script>

<style scoped>
/* ===== v1.1 AI 推荐体验 ===== */
.ai-spotlight {
  position: relative;
  border-radius: var(--radius-lg);
  padding: var(--space-lg) var(--space-xl);
  background: linear-gradient(135deg, var(--accent-color-light) 0%, transparent 100%);
  border: 1px solid var(--accent-color);
  overflow: hidden;
}
/* 右上角装饰 —— 低调的光晕点缀 */
.ai-spotlight::after {
  content: '';
  position: absolute;
  top: -40px;
  right: -40px;
  width: 160px;
  height: 160px;
  background: radial-gradient(circle, var(--accent-color) 0%, transparent 70%);
  opacity: 0.12;
  pointer-events: none;
}
.ai-spotlight-dismiss {
  position: absolute;
  top: 8px;
  right: 10px;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: var(--font-lg);
  line-height: 1;
  cursor: pointer;
  border-radius: 50%;
  z-index: 2;
  transition:
    background-color 0.15s,
    color 0.15s;
}
.ai-spotlight-dismiss:hover {
  background: var(--bg-secondary);
  color: var(--primary-text-color);
}
.ai-spotlight-body {
  position: relative;
  z-index: 1;
  display: flex;
  gap: var(--space-lg);
  align-items: flex-start;
}
.ai-spotlight-emoji {
  /* stylelint-disable-next-line declaration-property-value-allowed-list -- 装饰性 emoji 图标，2.5rem 明显大于 --font-2xl(2rem)，保留原尺寸观感 */
  font-size: 2.5rem;
  line-height: 1;
  flex-shrink: 0;
}
.ai-spotlight-text {
  flex: 1;
  min-width: 0;
}
.ai-spotlight-badge {
  display: inline-block;
  font-size: var(--font-xs);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  background: var(--card-bg-color);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-pill);
  border: 1px solid var(--accent-color);
  margin-bottom: var(--space-sm);
  letter-spacing: 0.5px;
}
.ai-spotlight-title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
  margin-bottom: var(--space-sm);
}
.ai-spotlight-desc {
  font-size: var(--font-sm);
  color: var(--secondary-text-color);
  line-height: 1.6;
  margin-bottom: var(--space-md);
  max-width: 35rem;
}
.ai-spotlight-actions {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  flex-wrap: wrap;
}
.ai-spotlight-link {
  color: var(--accent-color);
  font-size: var(--font-sm);
  text-decoration: none;
  font-weight: var(--weight-bold);
}
.ai-spotlight-link:hover {
  text-decoration: underline;
}

@media (--phone) {
  .ai-spotlight {
    padding: var(--space-lg);
  }
  .ai-spotlight-emoji {
    font-size: var(--font-2xl);
  }
  .ai-spotlight-body {
    gap: var(--space-md);
  }
  .ai-spotlight-title {
    font-size: var(--font-md);
  }
}
</style>
