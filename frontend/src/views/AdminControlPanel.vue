<template>
  <PageShell
    title="控制台"
    subtitle="系统设置、网络连接、安全管理、AI 视觉识别配置。"
    help="control-panel"
    width="content"
  >
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
            <router-link v-if="!step.done && step.to" :to="step.to" class="guide-link">
              前往 →
            </router-link>
            <span v-if="!step.done && step.hint" class="guide-hint">{{ step.hint }}</span>
          </div>
          <p class="guide-footer">完成以上步骤后，将平板放在摊位前即可开始使用</p>
        </div>
      </section>

      <!-- 局域网二维码 -->
      <SectionCard title="局域网连接" collapsible v-model:collapsed="qrCollapsed">
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
                <strong>所有设备连同一个 WiFi</strong
                >——装摊盒的主机、顾客用的平板、摊主看订单的手机，三台设备必须接入同一个无线网络。
              </li>
              <li>
                <strong>强烈推荐用手机开热点</strong>：漫展会场 WiFi
                常常拥堵或禁止设备互通，自己开个热点让主机 + 平板 + 手机都连上，稳定可控。
              </li>
              <li>
                用设备<strong>自带相机或浏览器</strong>扫码，打开后加入书签 /
                主屏幕快捷方式，方便下次直达。<span class="lan-guide__warn"
                  >微信/支付宝内扫可能拦截，请用系统相机。</span
                >
              </li>
            </ol>

            <details class="lan-guide__faq">
              <summary>扫码后无法连接？点击展开排障</summary>
              <ul class="lan-guide__faq-list">
                <li>
                  确认两台设备连的是<strong>同一个 WiFi 名称</strong
                  >（会场常有多个相近名字，别选错）
                </li>
                <li>
                  主机的<strong>防火墙</strong>需要放行 <code>5141</code> 端口（Windows
                  首次运行会弹出询问，选"允许专用/公用网络"）<br /><span class="lan-guide__warn"
                    >5140 是仅本机使用的回环端口，无需放行。</span
                  >
                </li>
                <li>
                  首次扫码会看到<strong>"您的连接不是私密连接"红屏警告</strong>——这是局域网自签证书的预期行为，点"高级
                  → 继续访问"即可，每台设备只需操作一次。详见
                  <code>docs/guide/lan-https.md</code>。
                </li>
                <li>
                  主机 IP 会在换网后变化 → 点下方「<strong>获取局域网二维码</strong>」刷新（变换 IP
                  后已接受过证书的设备会再警告一次）
                </li>
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
              <n-button
                type="primary"
                size="small"
                @click="handleCopy(entry.url, entry.label)"
                class="copy-btn"
              >
                点击复制链接
              </n-button>
            </div>
          </div>
        </n-space>
      </SectionCard>

      <!-- 安全设置 -->
      <SectionCard title="安全设置" collapsible v-model:collapsed="securityCollapsed">
        <div class="settings-grid">
          <div class="settings-card">
            <div class="settings-title">管理员密码</div>
            <n-form :model="adminForm" label-placement="top">
              <n-form-item label="旧密码">
                <n-input
                  v-model:value="adminForm.oldPassword"
                  type="password"
                  show-password-on="click"
                  placeholder="请输入旧密码"
                />
              </n-form-item>
              <n-form-item label="新密码 (至少 4 位)">
                <n-input
                  v-model:value="adminForm.newPassword"
                  type="password"
                  show-password-on="click"
                  placeholder="请输入新密码"
                />
              </n-form-item>
              <n-space justify="end">
                <n-button type="primary" :loading="adminSaving" @click="updateAdminPassword"
                  >保存</n-button
                >
              </n-space>
            </n-form>
            <n-alert v-if="adminMessage" :type="adminMessage.type" :bordered="false" class="mt-8">{{
              adminMessage.text
            }}</n-alert>
          </div>

          <div class="settings-card">
            <div class="settings-title">默认摊主密码（未配置摊主密码时采用）</div>
            <n-form :model="vendorForm" label-placement="top">
              <n-form-item label="新密码 (至少 4 位)">
                <n-input
                  v-model:value="vendorForm.newPassword"
                  type="password"
                  show-password-on="click"
                  placeholder="请输入新密码"
                />
              </n-form-item>
              <n-space justify="end">
                <n-button type="primary" :loading="vendorSaving" @click="updateVendorPassword"
                  >保存</n-button
                >
              </n-space>
            </n-form>
            <n-alert
              v-if="vendorMessage"
              :type="vendorMessage.type"
              :bordered="false"
              class="mt-8"
              >{{ vendorMessage.text }}</n-alert
            >
          </div>
        </div>
      </SectionCard>

      <!-- v1 历史数据：首启弹窗只弹一次，这里是那之后唯一的常驻导出出口 -->
      <SectionCard
        v-if="legacyHasBackup"
        title="历史数据（v1）"
        collapsible
        v-model:collapsed="legacyCollapsed"
      >
        <p class="legacy-desc">
          新模型没有迁移旧版的展会和订单，旧数据完整保留在
          <code>sale_system.db.v1-backup</code>，可以随时导出为 Excel。
        </p>
        <n-space align="center" :wrap="true">
          <n-button type="primary" :loading="legacyExporting" @click="handleLegacyExport">
            {{ legacyExporting ? '导出中…' : '导出旧数据为 Excel' }}
          </n-button>
          <span class="hint">
            备份中含 {{ legacyStatus?.event_count }} 个展会 / {{ legacyStatus?.order_count }} 张订单
          </span>
        </n-space>
      </SectionCard>

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
  </PageShell>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NSpace, NButton, NAlert, NForm, NFormItem, NInput } from 'naive-ui'
import QrcodeVue from 'qrcode.vue'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'
import { copyLink } from '@/services/clipboard'
import { useAuthStore } from '@/stores/authStore'
import VisionModelPanel from '@/components/product/VisionModelPanel.vue'
import { PageShell, SectionCard } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { exportLegacyXlsx } from '@/utils/legacyExport'

const fb = useFeedback()

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
  loadLegacyStatus()
})

// ===================== v1 历史数据 =====================
// 首启弹窗（MigrationNotice）只弹一次且立刻标记已读；若它不常驻，用户点完
// 「知道了」这台设备就再也导不出 v1 数据。这里是有备份时的常驻出口。
const legacyStatus = ref<Schemas['LegacyStatus'] | null>(null)
const legacyExporting = ref(false)
const legacyCollapsed = ref(false)

const legacyHasBackup = computed(() => legacyStatus.value?.has_backup === true)

async function loadLegacyStatus() {
  try {
    legacyStatus.value = await unwrap(api.GET('/legacy/status'))
  } catch (e) {
    // 读不到状态只是不显示这个 section，绝不能影响控制台主流程
    console.warn('[AdminControlPanel] 读取历史数据状态失败', e)
  }
}

async function handleLegacyExport() {
  legacyExporting.value = true
  try {
    const ok = await exportLegacyXlsx()
    if (ok) fb.success('导出成功')
  } catch (e) {
    console.error('下载旧数据失败:', e)
    fb.error(e, '下载失败')
  } finally {
    legacyExporting.value = false
  }
}

// ===================== 局域网 =====================
const isFetching = ref(false)
const fetchError = ref('')
const serverInfo = ref<Schemas['ServerInfo'] | null>(null)
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
    serverInfo.value = await unwrap(api.GET('/server-info'))
  } catch (e) {
    fetchError.value = errorMessage(e, '获取失败，请检查网络')
  } finally {
    isFetching.value = false
  }
}

async function handleCopy(url: string, label: string) {
  try {
    await copyLink(url)
    fb.success(`${label}链接已复制`)
  } catch {
    fb.error('复制失败')
  }
}

// ===================== 安全设置 =====================
const securityCollapsed = ref(false)
const adminForm = ref({ oldPassword: '', newPassword: '' })
const vendorForm = ref({ newPassword: '' })
const adminSaving = ref(false)
const vendorSaving = ref(false)
const adminMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)
const vendorMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

const authStore = useAuthStore()

async function updateAdminPassword() {
  const newPassword = adminForm.value.newPassword
  if (!adminForm.value.oldPassword) {
    adminMessage.value = { type: 'error', text: '请输入旧密码' }
    return
  }
  if (newPassword.length < 4) {
    adminMessage.value = { type: 'error', text: '新密码至少 4 位' }
    return
  }
  adminSaving.value = true
  adminMessage.value = null
  try {
    await unwrap(
      api.PUT('/admin/password', {
        body: {
          oldPassword: adminForm.value.oldPassword,
          newPassword,
        },
      })
    )
    adminMessage.value = { type: 'success', text: '管理员密码已更新' }
    adminForm.value = { oldPassword: '', newPassword: '' }
    // 密码改了需要重新登录
    await authStore.login(newPassword, 'admin')
  } catch (e) {
    adminMessage.value = {
      type: 'error',
      text: errorMessage(e, '修改失败'),
    }
  } finally {
    adminSaving.value = false
  }
}

async function updateVendorPassword() {
  if (vendorForm.value.newPassword.length < 4) {
    vendorMessage.value = { type: 'error', text: '新密码至少 4 位' }
    return
  }
  vendorSaving.value = true
  vendorMessage.value = null
  try {
    await unwrap(
      api.PUT('/admin/vendor-default-password', {
        body: { newPassword: vendorForm.value.newPassword },
      })
    )
    vendorMessage.value = { type: 'success', text: '默认摊主密码已更新' }
    vendorForm.value = { newPassword: '' }
  } catch (e) {
    vendorMessage.value = {
      type: 'error',
      text: errorMessage(e, '修改失败'),
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

/* 局域网 */
.qr-actions {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  flex-wrap: wrap;
}
.hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

/* ===== 局域网使用指南卡片 ===== */
.lan-guide {
  padding: var(--space-md) var(--space-lg);
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-left: 3px solid var(--accent-color);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-xs);
}
.lan-guide__head {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-sm);
}
.lan-guide__icon {
  font-size: var(--font-lg);
  line-height: 1;
}
.lan-guide__title {
  font-size: var(--font-md);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}
.lan-guide__intro {
  margin: 0 0 var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  background: var(--card-bg-color);
  border-radius: var(--radius-sm);
  font-size: var(--font-sm);
  line-height: 1.6;
  color: var(--secondary-text-color);
}
.lan-guide__intro strong {
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
}
.lan-guide__steps {
  margin: 0;
  padding-left: var(--space-lg);
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--secondary-text-color);
}
.lan-guide__steps li {
  margin-bottom: var(--space-sm);
}
.lan-guide__steps li:last-child {
  margin-bottom: 0;
}
.lan-guide__steps strong {
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
}
.lan-guide__warn {
  color: var(--warning-color);
  font-weight: var(--weight-medium);
}
.lan-guide__faq {
  margin-top: var(--space-md);
  padding-top: var(--space-sm);
  border-top: 1px dashed var(--border-color);
}
.lan-guide__faq > summary {
  cursor: pointer;
  font-size: var(--font-sm);
  font-weight: var(--weight-bold);
  color: var(--accent-color);
  list-style: none;
  user-select: none;
  padding: var(--space-xs) 0;
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
  margin: var(--space-sm) 0 0;
  padding-left: var(--space-lg);
  font-size: var(--font-sm);
  line-height: 1.7;
  color: var(--secondary-text-color);
}
.lan-guide__faq-list li {
  margin-bottom: var(--space-xs);
}
.lan-guide__faq-list code {
  background: var(--card-bg-color);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-sm);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: var(--font-xs);
  color: var(--accent-color);
}
.qr-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-lg);
  margin-top: var(--space-lg);
}
.qr-card {
  text-align: center;
  padding: var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
}
.qr-title {
  font-weight: var(--weight-bold);
  margin-bottom: var(--space-md);
  font-size: var(--font-md);
}
.qr-content {
  display: flex;
  justify-content: center;
  margin-bottom: var(--space-sm);
}
.qr-url {
  font-size: var(--font-sm);
  color: var(--text-muted);
  word-break: break-all;
  margin-bottom: var(--space-sm);
}
.copy-btn {
  width: 100%;
}

/* 安全设置 */
.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-xl);
}
@media (--phone) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
}
.settings-card {
  padding: var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
}
.settings-title {
  font-weight: var(--weight-bold);
  margin-bottom: var(--space-lg);
  font-size: var(--font-md);
}
.mt-8 {
  margin-top: var(--space-sm);
}

/* v1 历史数据 */
.legacy-desc {
  margin: 0 0 var(--space-lg);
  font-size: var(--font-base);
  line-height: 1.7;
  color: var(--secondary-text-color);
}
.legacy-desc code {
  background: var(--bg-elevated);
  padding: var(--space-xs);
  border-radius: var(--radius-sm);
}
</style>
