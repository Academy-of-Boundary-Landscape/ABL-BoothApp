<template>
  <PageShell title="展会管理" subtitle="创建和管理展会活动。" help="events" width="content">
    <main class="page-body">
      <n-space vertical size="large">
        <CreateEventForm />
        <EventList />
      </n-space>
    </main>
  </PageShell>
</template>

<script setup lang="ts">
import { NSpace } from 'naive-ui'
import { PageShell } from '@/components/ui'
import CreateEventForm from '@/components/event/CreateEventForm.vue'
import EventList from '@/components/event/EventList.vue'
</script>
