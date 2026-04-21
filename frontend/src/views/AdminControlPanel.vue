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
                <!-- 使用指南：三个二维码是给摊主自己的设备扫的，顾客只用摊主放在摊位上的平板 -->
                <div class="lan-guide">
                  <div class="lan-guide__head">
                    <span class="lan-guide__icon">📡</span>
                    <span class="lan-guide__title">怎么把摊盒铺到自己的设备上</span>
                  </div>

                  <p class="lan-guide__intro">
                    三个二维码是给<strong>摊主自己的其他设备</strong>扫的：把顾客点单页挂到平板上（摆在摊位给顾客点），把摊主页挂到手机上（实时看订单）。<strong>顾客不需要扫码</strong>，他们只用你摆好的平板。
                  </p>

                  <ol class="lan-guide__steps">
                    <li>
                      <strong>所有设备连同一个 WiFi</strong>——装摊盒的主机、顾客用的平板、摊主看订单的手机，三台设备必须接入同一个无线网络。
                    </li>
                    <li>
                      <strong>强烈推荐用手机开热点</strong>：漫展会场 WiFi 常常拥堵或禁止设备互通，自己开个热点让主机 + 平板 + 手机都连上，稳定可控。
                    </li>
                    <li>
                      用设备<strong>自带相机或浏览器</strong>扫码，打开后加入书签 / 主屏幕快捷方式，方便下次直达。<span class="lan-guide__warn">微信/支付宝内扫可能拦截，请用系统相机。</span>
                    </li>
                  </ol>

                  <details class="lan-guide__faq">
                    <summary>扫码后无法连接？点击展开排障</summary>
                    <ul class="lan-guide__faq-list">
                      <li>确认两台设备连的是<strong>同一个 WiFi 名称</strong>（会场常有多个相近名字，别选错）</li>
                      <li>主机的<strong>防火墙</strong>需要放行 <code>5140</code> 端口（Windows 首次运行会弹出询问，选"允许专用/公用网络"）</li>
                      <li>主机 IP 会在换网后变化 → 点下方「<strong>获取局域网二维码</strong>」刷新</li>
                      <li>部分校园网 / 酒店 WiFi 有"AP 隔离"禁止设备互通，换用手机热点</li>
                    </ul>
                  </details>
                </div>

                <div class="qr-actions">
                  <n-button type="primary" :loading="isFetching" @click="fetchServerInfo">
                    {{ isFetching ? '获取中...' : '获取局域网二维码' }}
                  </n-button>
                  <span class="hint">生成当前局域网的访问二维码，给顾客手机或摊主平板扫</span>
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

        <!-- v1.1 AI 拍照识别 推荐体验 -->
        <section v-if="showAiSpotlight" class="ai-spotlight">
          <button class="ai-spotlight-dismiss" aria-label="关闭" @click="dismissAiSpotlight">
            ×
          </button>
          <div class="ai-spotlight-body">
            <div class="ai-spotlight-emoji">📸</div>
            <div class="ai-spotlight-text">
              <div class="ai-spotlight-badge">v1.1 新功能</div>
              <div class="ai-spotlight-title">试试 AI 拍照识别</div>
              <div class="ai-spotlight-desc">
                让顾客拿手机对准商品拍张照，自动识别加入购物车。专为"帮朋友看摊 / 寄售"场景设计——不用贴条码、不用记 SKU，3 分钟就能跑起来。
              </div>
              <div class="ai-spotlight-actions">
                <n-button type="primary" @click="scrollToVisionPanel">
                  🚀 开始配置
                </n-button>
                <router-link to="/admin/help" class="ai-spotlight-link">
                  先看文档了解 →
                </router-link>
              </div>
            </div>
          </div>
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

// ===================== v1.1 AI 推荐体验 =====================
const AI_SPOTLIGHT_KEY = 'ai_spotlight_dismissed_v1.1'
const aiSpotlightDismissed = ref(localStorage.getItem(AI_SPOTLIGHT_KEY) === '1')

// 仅当用户未手动关闭、且 AI 视觉尚未就绪时显示
const showAiSpotlight = computed(() => !aiSpotlightDismissed.value && !visionReady.value)

function dismissAiSpotlight() {
  aiSpotlightDismissed.value = true
  localStorage.setItem(AI_SPOTLIGHT_KEY, '1')
}

function scrollToVisionPanel() {
  // VisionModelPanel 组件的根元素是 .vision-container
  const el = document.querySelector('.vision-container')
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

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
/* ===== v1.1 AI 推荐体验 ===== */
.ai-spotlight {
  position: relative;
  border-radius: var(--radius-lg, 12px);
  padding: 1.25rem 1.5rem;
  background: linear-gradient(
    135deg,
    var(--accent-color-light, rgba(99, 102, 241, 0.12)) 0%,
    var(--accent-color-lighter, rgba(99, 102, 241, 0.04)) 100%
  );
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
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  border-radius: 50%;
  z-index: 2;
  transition: background-color 0.15s, color 0.15s;
}
.ai-spotlight-dismiss:hover {
  background: var(--bg-secondary, rgba(0, 0, 0, 0.06));
  color: var(--primary-text-color);
}
.ai-spotlight-body {
  position: relative;
  z-index: 1;
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.ai-spotlight-emoji {
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
  font-size: 11px;
  font-weight: 700;
  color: var(--accent-color);
  background: var(--card-bg-color);
  padding: 2px 8px;
  border-radius: var(--radius-pill, 999px);
  border: 1px solid var(--accent-color);
  margin-bottom: 6px;
  letter-spacing: 0.5px;
}
.ai-spotlight-title {
  font-size: var(--font-lg);
  font-weight: 700;
  color: var(--primary-text-color);
  margin-bottom: 6px;
}
.ai-spotlight-desc {
  font-size: var(--font-sm);
  color: var(--secondary-text-color, var(--text-muted));
  line-height: 1.6;
  margin-bottom: 14px;
  max-width: 560px;
}
.ai-spotlight-actions {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}
.ai-spotlight-link {
  color: var(--accent-color);
  font-size: var(--font-sm);
  text-decoration: none;
  font-weight: 600;
}
.ai-spotlight-link:hover {
  text-decoration: underline;
}

@media (max-width: 480px) {
  .ai-spotlight { padding: 1rem; }
  .ai-spotlight-emoji { font-size: 2rem; }
  .ai-spotlight-body { gap: 12px; }
  .ai-spotlight-title { font-size: var(--font-md); }
}

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

/* ===== 局域网使用指南卡片 ===== */
.lan-guide {
  padding: 14px 16px;
  background: var(--bg-secondary, rgba(99, 102, 241, 0.04));
  border: 1px solid var(--border-color);
  border-left: 3px solid var(--accent-color);
  border-radius: var(--radius-md);
  margin-bottom: 4px;
}
.lan-guide__head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.lan-guide__icon {
  font-size: 1.2rem;
  line-height: 1;
}
.lan-guide__title {
  font-size: var(--font-md, 15px);
  font-weight: 700;
  color: var(--primary-text-color);
}
.lan-guide__intro {
  margin: 0 0 10px;
  padding: 10px 12px;
  background: var(--card-bg-color);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  line-height: 1.6;
  color: var(--secondary-text-color, var(--text-muted));
}
.lan-guide__intro strong {
  color: var(--primary-text-color);
  font-weight: 700;
}
.lan-guide__steps {
  margin: 0;
  padding-left: 1.25rem;
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--secondary-text-color, var(--text-muted));
}
.lan-guide__steps li {
  margin-bottom: 6px;
}
.lan-guide__steps li:last-child {
  margin-bottom: 0;
}
.lan-guide__steps strong {
  color: var(--primary-text-color);
  font-weight: 700;
}
.lan-guide__warn {
  color: var(--warning-color, #d08700);
  font-weight: 500;
}
.lan-guide__faq {
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px dashed var(--border-color);
}
.lan-guide__faq > summary {
  cursor: pointer;
  font-size: var(--font-sm);
  font-weight: 600;
  color: var(--accent-color);
  list-style: none;
  user-select: none;
  padding: 2px 0;
}
.lan-guide__faq > summary::-webkit-details-marker {
  display: none;
}
.lan-guide__faq > summary::before {
  content: '▸ ';
  display: inline-block;
  transition: transform 0.15s;
}
.lan-guide__faq[open] > summary::before {
  transform: rotate(90deg);
}
.lan-guide__faq-list {
  margin: 8px 0 0;
  padding-left: 1.25rem;
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--secondary-text-color, var(--text-muted));
}
.lan-guide__faq-list li {
  margin-bottom: 4px;
}
.lan-guide__faq-list code {
  background: var(--card-bg-color);
  padding: 1px 6px;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.92em;
  color: var(--accent-color);
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
