<!-- docs/.vitepress/theme/components/TimelineEn.vue -->
<script setup>
const timeline = [
  {
    type: 'error',
    time: 'Aug 2025',
    icon: '📖',
    title: 'Problem: chaotic paper records',
    content: 'Taking over a doujin club during summer, we faced messy inventory and paper-based bookkeeping — inspiring an automated tool.'
  },
  {
    type: 'warning',
    time: 'Sep–Nov 2025',
    icon: '🧪',
    title: 'Prototype: Web service (Flask)',
    content: 'Built a Flask + Vue prototype and deployed to a cloud server. It validated the workflow but hit a <b>heavy network dependency</b> wall.',
    link: {
      url: 'https://github.com/Renko6626/abl_sale_tool',
      text: 'View early code archive',
      title: 'See commits from 2025'
    }
  },
  {
    type: 'info',
    time: 'Nov 2025',
    icon: '💾',
    title: 'Attempt: Nuitka packaged app',
    content: 'Tried shipping the Python backend as a packaged app. Size bloat and runtime deps made it hard to run on regular member devices; laptops were inconvenient to carry.'
  },
  {
    type: 'success',
    time: 'Dec 2025 – Present',
    icon: '🚀',
    title: 'Refactor: Tauri local application',
    content: 'Rewrote the backend in <b>Rust</b>, built with <b>Tauri v2</b>. Achieved high performance, zero external deps, and fully offline on Windows/Android.'
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
        <h2 class="bk-title">Development Timeline</h2>
        <p class="bk-subtitle">From prototype website to a complete booth app</p>
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
      <small>🚩 Original project, continuously evolving</small>
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

/* Mobile */
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
