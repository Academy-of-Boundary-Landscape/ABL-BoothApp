<template>
  <div class="page">
    <header class="page-header">
      <div class="header-content">
        <div class="header-title-row">
          <h1>控制台</h1>
          <HelpBubble page="control-panel" />
        </div>
        <p>系统设置、网络连接、安全管理、AI 视觉识别配置。</p>
      </div>
    </header>

    <main class="page-body">
      <n-space vertical size="large">
        <!-- 快速开始引导 -->
        <section v-if="showGuide" class="guide-card">
          <div class="guide-header">
            <span class="guide-title">🚀 快速开始</span>
            <n-button text size="small" @click="dismissGuide">关闭</n-button>
          </div>

          <div class="guide-progress">
            <div class="guide-progress-bar">
              <div class="guide-progress-fill" :style="{ width: guideProgress + '%' }"></div>
            </div>
            <span class="guide-progress-text">{{ guideDoneCount }} / {{ guideTotalCount }} 完成</span>
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
              <router-link
                v-if="!step.done && step.to"
                :to="step.to"
                class="guide-link"
              >
                前往 →
              </router-link>
              <span v-if="!step.done && step.hint" class="guide-hint">{{ step.hint }}</span>
            </div>
            <p class="guide-footer">完成以上步骤后，将平板放在摊位前即可开始使用</p>
          </div>
        </section>

        <!-- 局域网二维码 -->
        <section>
          <div class="section-header" @click="qrCollapsed = !qrCollapsed">
            <h2>局域网连接</h2>
            <n-button text class="toggle-btn">
              {{ qrCollapsed ? '展开' : '折叠' }}
            </n-button>
          </div>
          <transition name="expand">
            <div v-show="!qrCollapsed" class="section-container">
              <n-space vertical size="small">
                <div class="qr-actions">
                  <n-button type="primary" :loading="isFetching" @click="fetchServerInfo">
                    {{ isFetching ? '获取中...' : '获取局域网二维码' }}
                  </n-button>
                  <span class="hint">同一局域网内扫码可直接访问对应页面，若无法连接请检查主机的防火墙设置</span>
                </div>
                <n-alert v-if="fetchError" type="error" :bordered="false">{{ fetchError }}</n-alert>
                <div v-if="serverInfo" class="qr-grid">
                  <div class="qr-card" v-for="entry in qrEntries" :key="entry.label">
                    <div class="qr-title">{{ entry.label }}</div>
                    <div class="qr-content">
                      <qrcode-vue :value="entry.url" :size="180" level="M" class="qr-code" />
                    </div>
                    <div class="qr-url">{{ entry.url }}</div>
                    <n-button type="primary" size="small" @click="handleCopy(entry.url, entry.label)" class="copy-btn">
                      点击复制链接
                    </n-button>
                  </div>
                </div>
              </n-space>
            </div>
          </transition>
        </section>

        <!-- 安全设置 -->
        <section>
          <div class="section-header" @click="securityCollapsed = !securityCollapsed">
            <h2>安全设置</h2>
            <n-button text class="toggle-btn">
              {{ securityCollapsed ? '展开' : '折叠' }}
            </n-button>
          </div>
          <transition name="expand">
            <div v-show="!securityCollapsed" class="section-container">
              <div class="settings-grid">
                <div class="settings-card">
                  <div class="settings-title">管理员密码</div>
                  <n-form :model="adminForm" label-placement="top">
                    <n-form-item label="旧密码">
                      <n-input v-model:value="adminForm.oldPassword" type="password" show-password-on="click" placeholder="请输入旧密码" />
                    </n-form-item>
                    <n-form-item label="新密码 (至少 4 位)">
                      <n-input v-model:value="adminForm.newPassword" type="password" show-password-on="click" placeholder="请输入新密码" />
                    </n-form-item>
                    <n-space justify="end">
                      <n-button type="primary" :loading="adminSaving" @click="updateAdminPassword">保存</n-button>
                    </n-space>
                  </n-form>
                  <n-alert v-if="adminMessage" :type="adminMessage.type" :bordered="false" class="mt-8">{{ adminMessage.text }}</n-alert>
                </div>

                <div class="settings-card">
                  <div class="settings-title">默认摊主密码（未配置摊主密码时采用）</div>
                  <n-form :model="vendorForm" label-placement="top">
                    <n-form-item label="新密码 (至少 4 位)">
                      <n-input v-model:value="vendorForm.newPassword" type="password" show-password-on="click" placeholder="请输入新密码" />
                    </n-form-item>
                    <n-space justify="end">
                      <n-button type="primary" :loading="vendorSaving" @click="updateVendorPassword">保存</n-button>
                    </n-space>
                  </n-form>
                  <n-alert v-if="vendorMessage" :type="vendorMessage.type" :bordered="false" class="mt-8">{{ vendorMessage.text }}</n-alert>
                </div>
              </div>
            </div>
          </transition>
        </section>

        <!-- AI 视觉识别 -->
        <VisionModelPanel />
      </n-space>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { NSpace, NButton, NAlert, NForm, NFormItem, NInput, useMessage } from 'naive-ui'
import QrcodeVue from 'qrcode.vue'
import api from '@/services/api'
import { copyLink } from '@/services/clipboard'
import { useAuthStore } from '@/stores/authStore'
import VisionModelPanel from '@/components/product/VisionModelPanel.vue'
import HelpBubble from '@/components/shared/HelpBubble.vue'

const message = useMessage()

// ===================== 快速开始引导 =====================
const guideDismissed = ref(localStorage.getItem('guide_dismissed') === 'true')
const hasEvents = ref(false)
const hasOngoingEvent = ref(false)
const hasProducts = ref(false)
const hasEventProducts = ref(false)
const visionReady = ref(false)

const showGuide = computed(() => !guideDismissed.value || !guideAllDone.value)

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
    done: false, // 无法自动检测，提示用户看下方
    hint: '↓ 见下方「局域网连接」',
  },
  {
    key: 'vision',
    label: '6. (可选) 配置 AI 拍照识别',
    done: visionReady.value,
    hint: '↓ 见下方「AI 视觉识别」',
  },
])

const guideTotalCount = computed(() => guideSteps.value.filter(s => s.key !== 'vision').length) // exclude optional
const guideDoneCount = computed(() => guideSteps.value.filter(s => s.key !== 'vision' && s.done).length)
const guideProgress = computed(() => guideTotalCount.value > 0 ? (guideDoneCount.value / guideTotalCount.value * 100) : 0)

const guideAllDone = computed(() =>
  hasEvents.value && hasProducts.value && hasEventProducts.value && hasOngoingEvent.value
)

function dismissGuide() {
  guideDismissed.value = true
  localStorage.setItem('guide_dismissed', 'true')
}

async function checkSetupStatus() {
  try {
    const [eventsRes, productsRes, visionRes] = await Promise.allSettled([
      api.get('/events'),
      api.get('/master-products'),
      api.get('/vision/status'),
    ])

    if (eventsRes.status === 'fulfilled') {
      const events = eventsRes.value.data || []
      hasEvents.value = events.length > 0
      hasOngoingEvent.value = events.some(e => e.status === '进行中')
      // 检查是否有展会已上架商品：取第一个展会的商品列表
      if (events.length > 0) {
        try {
          const { data } = await api.get(`/events/${events[0].id}/products`)
          hasEventProducts.value = (data || []).length > 0
        } catch { /* ignore */ }
      }
    }

    if (productsRes.status === 'fulfilled') {
      hasProducts.value = (productsRes.value.data || []).length > 0
    }

    if (visionRes.status === 'fulfilled') {
      visionReady.value = visionRes.value.data?.is_ready === true
    }
  } catch { /* ignore */ }
}

onMounted(() => {
  checkSetupStatus()
})

// ===================== 局域网 =====================
const isFetching = ref(false)
const fetchError = ref('')
const serverInfo = ref(null)
const qrCollapsed = ref(false)

const qrEntries = computed(() => {
  if (!serverInfo.value) return []
  return [
    { label: '顾客入口', url: serverInfo.value.order_url },
    { label: '摊主入口', url: serverInfo.value.vendor_url },
    { label: '管理员入口', url: serverInfo.value.admin_url },
  ]
})

async function fetchServerInfo() {
  isFetching.value = true
  fetchError.value = ''
  try {
    const { data } = await api.get('/server-info')
    serverInfo.value = data
  } catch (e) {
    fetchError.value = e.response?.data?.error || '获取失败，请检查网络'
  } finally {
    isFetching.value = false
  }
}

async function handleCopy(url, label) {
  try {
    await copyLink(url)
    message.success(`${label}链接已复制`)
  } catch {
    message.error('复制失败')
  }
}

// ===================== 安全设置 =====================
const securityCollapsed = ref(false)
const adminForm = ref({ oldPassword: '', newPassword: '' })
const vendorForm = ref({ newPassword: '' })
const adminSaving = ref(false)
const vendorSaving = ref(false)
const adminMessage = ref(null)
const vendorMessage = ref(null)

const authStore = useAuthStore()

async function updateAdminPassword() {
  adminSaving.value = true
  adminMessage.value = null
  try {
    await api.put('/auth/admin-password', {
      old_password: adminForm.value.oldPassword,
      new_password: adminForm.value.newPassword,
    })
    adminMessage.value = { type: 'success', text: '管理员密码已更新' }
    adminForm.value = { oldPassword: '', newPassword: '' }
    // 密码改了需要重新登录
    await authStore.login(adminForm.value.newPassword, 'admin')
  } catch (e) {
    adminMessage.value = {
      type: 'error',
      text: e.response?.data?.error || '修改失败',
    }
  } finally {
    adminSaving.value = false
  }
}

async function updateVendorPassword() {
  vendorSaving.value = true
  vendorMessage.value = null
  try {
    await api.put('/auth/vendor-password', {
      new_password: vendorForm.value.newPassword,
    })
    vendorMessage.value = { type: 'success', text: '默认摊主密码已更新' }
    vendorForm.value = { newPassword: '' }
  } catch (e) {
    vendorMessage.value = {
      type: 'error',
      text: e.response?.data?.error || '修改失败',
    }
  } finally {
    vendorSaving.value = false
  }
}
</script>

<style scoped>
/* ===== 快速开始引导 ===== */
.guide-card {
  background: var(--card-bg-color);
  border: 2px solid var(--accent-color);
  border-radius: var(--radius-md);
  padding: 1rem 1.25rem;
}
.guide-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}
.guide-title {
  font-size: var(--font-lg);
  font-weight: 700;
}
.guide-done {
  font-size: var(--font-md);
  font-weight: 600;
  color: var(--accent-color);
  padding: 0.5rem 0;
}
.guide-steps {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.guide-step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
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
  font-weight: 500;
  color: var(--primary-text-color);
}
.guide-link {
  font-size: var(--font-sm);
  font-weight: 600;
  color: var(--accent-color);
  text-decoration: none;
  margin-left: auto;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--accent-color);
  transition: all 0.15s;
  white-space: nowrap;
}
.guide-link:hover {
  background: var(--accent-color);
  color: white;
}
.guide-hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin-left: auto;
  white-space: nowrap;
}
.guide-footer {
  margin: 0.75rem 0 0;
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.guide-progress {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}
.guide-progress-bar {
  flex: 1;
  height: 6px;
  background: var(--border-color);
  border-radius: 3px;
  overflow: hidden;
}
.guide-progress-fill {
  height: 100%;
  background: var(--accent-color);
  border-radius: 3px;
  transition: width 0.5s ease;
}
.guide-progress-text {
  font-size: var(--font-sm);
  font-weight: 600;
  color: var(--text-muted);
  white-space: nowrap;
}

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

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  user-select: none;
  padding: 0.75rem 1rem;
  background: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  transition: all 0.2s ease;
  margin-bottom: 0.5rem;
}
.section-header:hover {
  border-color: var(--accent-color);
}
.section-header h2 {
  margin: 0;
  font-size: var(--font-lg);
  color: var(--accent-color);
  font-weight: 600;
}
.toggle-btn {
  font-size: var(--font-base);
  color: var(--accent-color);
}

.section-container {
  background-color: var(--card-bg-color);
  border: 2px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 1.5rem;
}

/* 局域网 */
.qr-actions {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}
.hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.qr-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 1rem;
  margin-top: 1rem;
}
.qr-card {
  text-align: center;
  padding: 1rem;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
}
.qr-title {
  font-weight: 600;
  margin-bottom: 0.75rem;
  font-size: var(--font-md);
}
.qr-content {
  display: flex;
  justify-content: center;
  margin-bottom: 0.5rem;
}
.qr-url {
  font-size: var(--font-sm);
  color: var(--text-muted);
  word-break: break-all;
  margin-bottom: 0.5rem;
}
.copy-btn {
  width: 100%;
}

/* 安全设置 */
.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
}
@media (max-width: 768px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
}
.settings-card {
  padding: 1rem;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
}
.settings-title {
  font-weight: 600;
  margin-bottom: 1rem;
  font-size: var(--font-md);
}
.mt-8 {
  margin-top: 0.5rem;
}

/* 折叠动画 */
.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}
.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
}
.expand-enter-to,
.expand-leave-from {
  max-height: 2000px;
  opacity: 1;
}
</style>
