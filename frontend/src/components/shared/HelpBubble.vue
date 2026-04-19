<template>
  <n-popover
    trigger="click"
    placement="bottom-end"
    :width="300"
    class="help-popover"
  >
    <template #trigger>
      <button class="help-trigger" title="页面帮助">?</button>
    </template>
    <div class="help-content">
      <div class="help-title">{{ content.title }}</div>
      <ul class="help-tips">
        <li v-for="(tip, i) in content.tips" :key="i">{{ tip }}</li>
      </ul>
      <router-link to="/admin/help" class="help-more">查看完整教程 →</router-link>
    </div>
  </n-popover>
</template>

<script setup>
import { computed } from 'vue'
import { NPopover } from 'naive-ui'
import { helpContent } from '@/config/helpContent'

const props = defineProps({
  page: { type: String, required: true },
})

const content = computed(() => helpContent[props.page] || { title: '帮助', tips: [] })
</script>

<style scoped>
.help-trigger {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1.5px solid var(--border-color);
  background: var(--card-bg-color);
  color: var(--text-muted);
  font-size: 14px;
  font-weight: 700;
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
  padding: 4px 0;
}

.help-title {
  font-size: var(--font-md, 14px);
  font-weight: 700;
  color: var(--primary-text-color);
  margin-bottom: 10px;
}

.help-tips {
  list-style: none;
  padding: 0;
  margin: 0 0 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.help-tips li {
  font-size: var(--font-sm, 13px);
  color: var(--text-color);
  line-height: 1.5;
  padding-left: 16px;
  position: relative;
}
.help-tips li::before {
  content: '·';
  position: absolute;
  left: 4px;
  color: var(--accent-color);
  font-weight: 700;
}

.help-more {
  display: block;
  font-size: var(--font-sm, 13px);
  color: var(--accent-color);
  text-decoration: none;
  font-weight: 600;
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
}
.help-more:hover {
  text-decoration: underline;
}
</style>
