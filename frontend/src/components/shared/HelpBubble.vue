<template>
  <n-popover trigger="click" placement="bottom-end" :width="300" class="help-popover">
    <template #trigger>
      <button class="help-trigger" title="页面帮助">?</button>
    </template>
    <div class="help-content">
      <div class="help-title">{{ content.title }}</div>
      <ul class="help-tips">
        <!-- tips 是 helpContent.js 里的静态文案（非用户输入），仅支持 **粗体** 简易 Markdown -->
        <li v-for="(tip, i) in content.tips" :key="i" v-html="renderTip(tip)"></li>
      </ul>
      <router-link to="/admin/help" class="help-more">查看完整教程 →</router-link>
    </div>
  </n-popover>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NPopover } from 'naive-ui'
import { helpContent } from '@/config/helpContent'

const props = defineProps<{ page: string }>()

const content = computed(() => helpContent[props.page] || { title: '帮助', tips: [] })

// 先转义 HTML 特殊字符，再把 **xxx** 替换为 <strong>xxx</strong>
// 内容来自仓库内静态 JS 文件，不是用户输入，但仍做 escape 防御
function renderTip(raw: string) {
  const escaped = String(raw)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
  return escaped.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
}
</script>

<style scoped>
.help-trigger {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1.5px solid var(--border-color);
  background: var(--card-bg-color);
  color: var(--text-muted);
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  flex-shrink: 0;
}
.help-trigger:hover {
  border-color: var(--accent-color);
  color: var(--accent-color);
}

.help-content {
  padding: var(--space-xs) 0;
}

.help-title {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
  margin-bottom: var(--space-sm);
}

.help-tips {
  list-style: none;
  padding: 0;
  margin: 0 0 var(--space-md);
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.help-tips li {
  font-size: var(--font-sm);
  color: var(--primary-text-color);
  line-height: 1.5;
  padding-left: var(--space-lg);
  position: relative;
}
.help-tips li::before {
  content: '·';
  position: absolute;
  left: var(--space-xs);
  color: var(--accent-color);
  font-weight: var(--weight-bold);
}
.help-tips li :deep(strong) {
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
}

.help-more {
  display: block;
  font-size: var(--font-sm);
  color: var(--accent-color);
  text-decoration: none;
  font-weight: var(--weight-bold);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border-color);
}
.help-more:hover {
  text-decoration: underline;
}
</style>
