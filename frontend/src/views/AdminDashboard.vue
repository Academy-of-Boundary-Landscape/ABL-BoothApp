<template>
  <PageShell title="展会管理" subtitle="创建和管理展会活动。" help="events" width="content">
    <main class="page-body">
      <n-space vertical size="large">
        <!-- 快速开始引导（原控制台，搬到 /admin 的落地页） -->
        <section v-if="showGuide" class="guide-card">
          <div class="guide-header">
            <span class="guide-title">🚀 快速开始</span>
            <n-button text size="small" @click="dismissGuide">关闭</n-button>
          </div>

          <div class="guide-progress">
            <div class="guide-progress-bar">
              <div class="guide-progress-fill" :style="{ width: guideProgress + '%' }"></div>
            </div>
            <span class="guide-progress-text"
              >{{ guideDoneCount }} / {{ guideTotalCount }} 完成</span
            >
          </div>

          <div v-if="guideAllDone" class="guide-done">
            🎉 一切就绪！你可以将平板放在摊位前，开始接待顾客了。
          </div>

          <div v-else class="guide-steps">
            <div
              v-for="step in guideSteps"
              :key="step.key"
              class="guide-step"
              :class="{ 'guide-step--done': step.done }"
            >
              <span class="guide-check">{{ step.done ? '✅' : '⬜' }}</span>
              <span class="guide-text">{{ step.label }}</span>
              <router-link v-if="!step.done && step.to" :to="step.to" class="guide-link">
                前往 →
              </router-link>
              <span v-if="!step.done && step.hint" class="guide-hint">{{ step.hint }}</span>
            </div>
            <p class="guide-footer">完成以上步骤后，将平板放在摊位前即可开始使用</p>
          </div>
        </section>

        <CreateEventForm />
        <EventList />
      </n-space>
    </main>
  </PageShell>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NButton, NSpace } from 'naive-ui'
import { api, unwrap } from '@/api/client'
import { PageShell } from '@/components/ui'
import CreateEventForm from '@/components/event/CreateEventForm.vue'
import EventList from '@/components/event/EventList.vue'

// ===================== 快速开始引导（原 AdminControlPanel 逻辑原样搬）=====================
const guideDismissed = ref(localStorage.getItem('guide_dismissed') === 'true')
const hasEvents = ref(false)
const hasOngoingEvent = ref(false)
const hasProducts = ref(false)
const hasEventProducts = ref(false)
const visionReady = ref(false)

const showGuide = computed(() => !guideDismissed.value || !guideAllDone.value)

// 指向旧控制台的步骤已改指设置页；局域网步骤带 `#lan` 锚点（LanSettings 根元素 id="lan"）。
const guideSteps = computed(() => [
  {
    key: 'event',
    label: '1. 创建展会',
    done: hasEvents.value,
    to: '/admin/events',
  },
  {
    key: 'products',
    label: '2. 添加全局商品',
    done: hasProducts.value,
    to: '/admin/master-products',
  },
  {
    key: 'event-products',
    label: '3. 为展会上架商品',
    done: hasEventProducts.value,
    to: hasEvents.value ? null : '/admin/events',
    hint: hasEvents.value ? '在展会管理中点击展会进入商品管理' : '请先创建展会',
  },
  {
    key: 'ongoing',
    label: '4. 将展会状态改为「进行中」',
    done: hasOngoingEvent.value,
    to: '/admin/events',
  },
  {
    key: 'qr',
    label: '5. 获取局域网二维码',
    done: false, // 无法自动检测，链接到设置页的局域网区块
    to: '/admin/settings#lan',
  },
  {
    key: 'vision',
    label: '6. (可选) 配置 AI 拍照识别',
    done: visionReady.value,
    to: '/admin/settings',
  },
])

const guideTotalCount = computed(() => guideSteps.value.filter((s) => s.key !== 'vision').length) // exclude optional
const guideDoneCount = computed(
  () => guideSteps.value.filter((s) => s.key !== 'vision' && s.done).length
)
const guideProgress = computed(() =>
  guideTotalCount.value > 0 ? (guideDoneCount.value / guideTotalCount.value) * 100 : 0
)

const guideAllDone = computed(
  () => hasEvents.value && hasProducts.value && hasEventProducts.value && hasOngoingEvent.value
)

function dismissGuide() {
  guideDismissed.value = true
  localStorage.setItem('guide_dismissed', 'true')
}

async function checkSetupStatus() {
  try {
    const [eventsRes, productsRes, visionRes] = await Promise.allSettled([
      unwrap(api.GET('/events')),
      unwrap(api.GET('/master-products')),
      unwrap(api.GET('/vision/status')),
    ])

    if (eventsRes.status === 'fulfilled') {
      const events = eventsRes.value || []
      hasEvents.value = events.length > 0
      hasOngoingEvent.value = events.some((e) => e.status === '进行中')
      // 检查是否有展会已上架商品：取第一个展会的商品列表
      if (events.length > 0) {
        try {
          const eventProducts = await unwrap(
            api.GET('/events/{event_id}/products', {
              params: { path: { event_id: events[0].id } },
            })
          )
          hasEventProducts.value = (eventProducts || []).length > 0
        } catch {
          /* ignore */
        }
      }
    }

    if (productsRes.status === 'fulfilled') {
      hasProducts.value = (productsRes.value || []).length > 0
    }

    if (visionRes.status === 'fulfilled') {
      visionReady.value = visionRes.value?.is_ready === true
    }
  } catch {
    /* ignore */
  }
}

onMounted(() => {
  checkSetupStatus()
})
</script>

<style scoped>
/* ===== 快速开始引导 ===== */
.guide-card {
  background: var(--card-bg-color);
  border: 2px solid var(--accent-color);
  border-radius: var(--radius-md);
  padding: var(--space-lg) var(--space-xl);
}
.guide-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-md);
}
.guide-title {
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
}
.guide-done {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  padding: var(--space-sm) 0;
}
.guide-steps {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.guide-step {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm);
  border-radius: var(--radius-sm);
  font-size: var(--font-base);
  transition: background 0.15s;
}
.guide-step:hover {
  background: var(--bg-secondary);
}
.guide-step--done {
  opacity: 0.6;
}
.guide-check {
  flex-shrink: 0;
  font-size: var(--font-md);
}
.guide-text {
  font-weight: var(--weight-medium);
  color: var(--primary-text-color);
}
.guide-link {
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  text-decoration: none;
  margin-left: auto;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-pill);
  border: 1px solid var(--accent-color);
  transition: all 0.15s;
  white-space: nowrap;
}
.guide-link:hover {
  background: var(--accent-color);
  color: var(--text-white);
}
.guide-hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-left: auto;
  white-space: nowrap;
}
.guide-footer {
  margin: var(--space-md) 0 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.guide-progress {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-md);
}
.guide-progress-bar {
  flex: 1;
  height: 6px;
  background: var(--border-color);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.guide-progress-fill {
  height: 100%;
  background: var(--accent-color);
  border-radius: var(--radius-sm);
  transition: width 0.5s ease;
}
.guide-progress-text {
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  color: var(--text-muted);
  white-space: nowrap;
}
</style>
