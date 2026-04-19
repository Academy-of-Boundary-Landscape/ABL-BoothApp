<template>
  <div class="page">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>展会管理</h1>
          <HelpBubble page="events" />
        </div>
        <p>创建和管理展会活动。</p>
      </div>
    </header>

    <main class="page-body">
      <n-space vertical size="large">
        <CreateEventForm />
        <EventList />
      </n-space>
    </main>
  </div>
</template>

<script setup>
import { NSpace } from 'naive-ui'
import CreateEventForm from '@/components/event/CreateEventForm.vue'
import EventList from '@/components/event/EventList.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'
</script>

<style scoped>
.page {
  max-width: 960px;
}

.header-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.page-header {
  margin-bottom: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem;
  font-size: var(--font-xl);
  color: var(--accent-color);
}
.page-header p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--font-base);
}
</style>
