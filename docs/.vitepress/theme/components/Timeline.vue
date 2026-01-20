<!-- docs/.vitepress/theme/components/Timeline.vue -->
<script setup>
const timeline = [
  {
    type: 'error',
    time: '2025年8月',
    icon: '📖',
    title: '痛点：混乱的纸质记录',
    content: '暑假接手一个同人社团主催后，面对混乱库存和纸质账目的痛点，萌生了开发自动化工具的想法。'
  },
  {
    type: 'warning',
    time: '2025年9月 - 11月',
    icon: '🧪',
    title: '原型：Web 服务版 (Flask)',
    content: '开发 Flask + Vue 原型并部署在云服务器。虽确立了核心工作流，但遇到了<b>严重依赖网络</b>的瓶颈。',
    link: {
      url: 'https://github.com/Renko6626/abl_sale_tool',
      text: '查看早期代码存档',
      title: '点击查看 2025 年的提交记录'
    }
  },
  {
    type: 'info',
    time: '2025年11月',
    icon: '💾',
    title: '尝试：Nuitka 打包版',
    content: '尝试将 Python 后端打包分发。受限于体积臃肿与运行库依赖，难以在普通社团成员的电脑上运行，且Windows电脑难以携带。'
  },
  {
    type: 'success',
    time: '2025年12月 - 至今',
    icon: '🚀',
    title: '重构：Tauri 本地应用',
    content: '使用 <b>Rust</b> 重写后端，基于 <b>Tauri v2</b> 构建。实现了 Win/Android 双平台高性能、零依赖、离线运行。'
  }
]
</script>

<template>
  <div class="bk-timeline-wrapper">
    <div class="bk-timeline-header">
      <div class="bk-icon-box">
        <span class="bk-header-icon">📖</span>
      </div>
      <div class="bk-header-text">
        <h2 class="bk-title">开发历程</h2>
        <p class="bk-subtitle">从原型网站到完整的出摊APP</p>
      </div>
    </div>

    <div class="bk-timeline-body">
      <div class="bk-timeline">
        <div 
          v-for="(item, index) in timeline" 
          :key="index"
          class="bk-timeline-item"
          :class="`bk-type-${item.type}`"
        >
          <div class="bk-timeline-line"></div>
          <div class="bk-timeline-icon">
            <span class="bk-icon">{{ item.icon }}</span>
          </div>
          <div class="bk-timeline-content">
            <div class="bk-timeline-time">{{ item.time }}</div>
            <div class="bk-timeline-header">
              <span class="bk-tl-title">{{ item.title }}</span>
              <a 
                v-if="item.link"
                :href="item.link.url" 
                target="_blank" 
                class="bk-archive-link"
                :title="item.link.title"
              >
                <span class="bk-link-icon">🏛️</span> {{ item.link.text }}
              </a>
            </div>
            <p class="bk-tl-content" v-html="item.content"></p>
          </div>
        </div>
      </div>
    </div>
    
    <div class="bk-timeline-footer">
      <small>🚩 始于原创，持续进化</small>
    </div>
  </div>
</template>

<style scoped>
.bk-timeline-wrapper {
  margin: 4rem auto;
  max-width: 800px;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  padding: 2rem;
  box-shadow: 0 4px 24px rgba(0,0,0,0.04);
}

.bk-timeline-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 2rem;
  padding-bottom: 1rem;
  border-bottom: 1px dashed var(--vp-c-divider);
}

.bk-icon-box {
  background: var(--vp-c-bg);
  padding: 8px;
  border-radius: 10px;
  border: 1px solid var(--vp-c-divider);
  display: flex;
}

.bk-header-icon {
  font-size: 24px;
  line-height: 1;
}

.bk-title {
  margin: 0 !important;
  font-size: 1.4rem;
  font-weight: 700;
  border: none !important;
  line-height: 1.2;
}

.bk-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.9rem;
  color: var(--vp-c-text-2);
}

.bk-timeline-body {
  padding: 0 0.5rem;
}

.bk-timeline {
  position: relative;
}

.bk-timeline-item {
  position: relative;
  padding-left: 3rem;
  padding-bottom: 2rem;
}

.bk-timeline-item:last-child {
  padding-bottom: 0;
}

.bk-timeline-item:last-child .bk-timeline-line {
  display: none;
}

.bk-timeline-line {
  position: absolute;
  left: 1rem;
  top: 2rem;
  bottom: 0;
  width: 2px;
  background: var(--vp-c-divider);
}

.bk-timeline-icon {
  position: absolute;
  left: 0;
  top: 0;
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  z-index: 1;
}

.bk-type-error .bk-timeline-icon {
  border-color: #d03050;
  background: rgba(208, 48, 80, 0.1);
}

.bk-type-warning .bk-timeline-icon {
  border-color: #f0a020;
  background: rgba(240, 160, 32, 0.1);
}

.bk-type-info .bk-timeline-icon {
  border-color: #3451b2;
  background: rgba(52, 81, 178, 0.1);
}

.bk-type-success .bk-timeline-icon {
  border-color: #18a058;
  background: rgba(24, 160, 88, 0.1);
}

.bk-icon {
  font-size: 1rem;
  line-height: 1;
}

.bk-timeline-content {
  padding-top: 0.1rem;
}

.bk-timeline-time {
  font-size: 0.85rem;
  color: var(--vp-c-text-3);
  margin-bottom: 0.3rem;
  font-weight: 500;
}

.bk-timeline-header {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.8rem;
  margin-bottom: 0.5rem;
}

.bk-tl-title {
  font-weight: 700;
  font-size: 1.05rem;
  color: var(--vp-c-text-1);
}

.bk-tl-content {
  margin: 0.25rem 0 0 0;
  font-size: 0.95rem;
  line-height: 1.6;
  color: var(--vp-c-text-2);
}

.bk-timeline-footer {
  margin-top: 1.5rem;
  text-align: right;
  color: var(--vp-c-text-3);
  font-weight: 500;
  opacity: 0.7;
}

.bk-archive-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 0.8rem;
  text-decoration: none;
  background: var(--vp-c-bg-alt);
  border: 1px solid var(--vp-c-divider);
  padding: 2px 8px;
  border-radius: 4px;
  color: var(--vp-c-text-2);
  transition: all 0.2s ease;
}

.bk-archive-link:hover {
  border-color: var(--vp-c-brand);
  color: var(--vp-c-brand);
  background: var(--vp-c-bg);
}

/* 移动端适配 */
@media (max-width: 640px) {
  .bk-timeline-wrapper {
    padding: 1.5rem 1rem;
    margin: 2rem 0;
    border-radius: 12px;
  }
  
  .bk-timeline-item {
    padding-left: 2.5rem;
  }
  
  .bk-timeline-line {
    left: 0.75rem;
  }
  
  .bk-timeline-icon {
    width: 1.5rem;
    height: 1.5rem;
  }
  
  .bk-icon {
    font-size: 0.85rem;
  }
}
</style>