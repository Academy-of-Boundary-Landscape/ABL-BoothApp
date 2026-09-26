<template>
  <div id="about" class="about-settings">
    <n-space vertical size="large">
      <!-- 关于摊盒 -->
      <SectionCard title="关于摊盒">
        <div class="header-section">
          <n-avatar
            :size="100"
            src="/logo.png"
            fallback-src="https://via.placeholder.com/96?text=THO"
            class="logo"
            bordered
          />
          <h2 class="app-title">摊盒 Booth-Kernel</h2>
          <p class="app-subtitle">基于 Tauri 构建的现代化出摊一体工具，旨在改进同人摊主工作流</p>

          <n-flex justify="center" size="small" class="tag-row">
            <n-tag :bordered="false" type="info" round size="small">
              <template #icon
                ><n-icon><LogoTux /></n-icon
              ></template>
              Tauri v2
            </n-tag>
            <n-tag :bordered="false" type="primary" round size="small">
              <template #icon
                ><n-icon><LogoWindows /></n-icon
              ></template>
              Windows
            </n-tag>
            <n-tag :bordered="false" type="success" round size="small">
              <template #icon
                ><n-icon><LogoAndroid /></n-icon
              ></template>
              Android
            </n-tag>
            <n-tag :bordered="false" type="warning" round size="small">MIT License</n-tag>
          </n-flex>

          <div class="update-row">
            <n-button secondary @click="showUpdateModal = true">检查更新</n-button>
          </div>
        </div>

        <n-divider />
        <n-blockquote>
          “同人展会的魅力在于创作者与同好之间的交流。我不希望摊主们被杂乱的账本、卡顿的网络和繁琐的计算束缚。
          该工具的初衷，是把‘出摊’变得更简单、更优雅，让摊主能更多享受展会的快乐，而非被琐碎事项所束缚。”
        </n-blockquote>
      </SectionCard>

      <!-- 项目时间线（默认折叠） -->
      <SectionCard title="项目时间线" collapsible collapsed>
        <n-timeline size="large">
          <n-timeline-item type="error" title="痛点：混乱的纸质记录" time="2025年8月">
            <span class="text-muted"
              >暑假接手社团主催后，面对混乱库存和纸质账目的痛点，决定开发自动化工具。</span
            >
          </n-timeline-item>
          <n-timeline-item type="warning" title="尝试：Web 服务版" time="2025年9月-11月">
            <span class="text-muted"
              >开发 Flask+Vue 的原型并部署在社团云服务器，确立了核心工作流。</span
            >
          </n-timeline-item>
          <n-timeline-item type="warning" title="发布: Nuitka打包版" time="2025年11月">
            <span class="text-muted"
              >尝试朴素地把Flask给打包，但受限于 Python 后端体积与平台限制难以广泛分发。</span
            >
          </n-timeline-item>
          <n-timeline-item type="success" title="v1.0：Tauri 本地应用" time="2025年12月">
            <span class="text-muted"
              >使用 Rust 重写后端，Tauri v2 构建，实现 Win/Android 双平台高性能体验。</span
            >
          </n-timeline-item>
          <n-timeline-item type="info" title="v1.1：AI 拍照识别 + 体验打磨" time="2026年4月">
            <span class="text-muted"
              >引入针对二次元同人场景优化的 AI 图像识别（ONNX Runtime
              驱动），新增多维标签系统、管理端引导、闲置吸引屏、售罄置底等上百处体验打磨；数据库层重构并发安全，SQLite
              WAL + 原子事务，多人同时下单不再丢数据。</span
            >
          </n-timeline-item>
          <n-timeline-item type="success" title="v1.2：复式账本" time="2026年9月">
            <span class="text-muted"
              >账本重写为复式记账，货和钱都有来处和去处；新增套装与最优折扣、收摊向导与结算单（寄售分账）、
              退货 / 赠送 / 报废、条码扫描与扫码枪；管理后台按展会生命周期重新组织，摊主端改为订单 /
              库存 / 收摊三个 tab。</span
            >
          </n-timeline-item>
        </n-timeline>
      </SectionCard>

      <!-- 核心特征 -->
      <SectionCard title="核心特征">
        <n-grid x-gap="16" y-gap="16" cols="1 s:1 m:3" responsive="screen">
          <n-grid-item v-for="feature in features" :key="feature.title">
            <n-card
              class="feature-card"
              content-style="padding: var(--space-lg);"
              hoverable
              embedded
            >
              <n-flex align="center" :wrap="false" class="feature-header">
                <n-icon size="26" :color="feature.color">
                  <component :is="feature.icon" />
                </n-icon>
                <span class="feature-title">{{ feature.title }}</span>
              </n-flex>
              <p class="feature-desc">{{ feature.desc }}</p>
            </n-card>
          </n-grid-item>
        </n-grid>
      </SectionCard>

      <!-- 技术架构 -->
      <SectionCard title="技术架构">
        <n-flex size="small" class="mb-2">
          <n-tag :bordered="false" type="success">Vue 3 + Naive UI</n-tag>
          <n-tag :bordered="false" type="info">Rust (Axum)</n-tag>
          <n-tag :bordered="false" type="default">SQLite</n-tag>
          <n-tag :bordered="false" type="warning">Tauri v2</n-tag>
        </n-flex>
        <p class="text-muted">
          Naive UI 提供现代化交互，Rust 负责高性能 HTTP 服务与业务逻辑，SQLite 确保数据本地化存储。
        </p>
      </SectionCard>

      <!-- 致谢 -->
      <SectionCard title="致谢">
        <n-grid x-gap="12" y-gap="12" cols="1 s:2" responsive="screen">
          <n-grid-item v-for="item in credits" :key="item.title">
            <div class="credit-item">
              <div class="credit-title">{{ item.title }}</div>
              <div class="credit-desc">{{ item.desc }}</div>
            </div>
          </n-grid-item>
        </n-grid>
      </SectionCard>

      <!-- 指南 & 声明 -->
      <SectionCard title="指南 & 声明">
        <n-collapse arrow-placement="right" :default-expanded-names="['free', 'privacy']">
          <n-collapse-item name="free">
            <template #header>
              <n-flex align="center" size="small">
                <n-icon color="#d03050"><GiftOutline /></n-icon>
                <span style="font-weight: var(--weight-bold); color: var(--n-text-color)"
                  >永久免费声明</span
                >
              </n-flex>
            </template>
            <div class="collapse-inner">
              <n-alert type="error" :show-icon="false" class="mb-2">
                <strong>🚫 谨防诈骗</strong>
                <br />
                本软件为开源项目，<strong>永远免费</strong>。市面上任何出售本软件、源码或“代部署”的行为均为诈骗。
              </n-alert>
              <p class="text-muted">
                如果您是在任何地方付费获取该软件，请立即申请退款并举报。 您永远可以在 GitHub
                或官网免费下载最新版，并直接向开发者反馈问题。
              </p>
            </div>
          </n-collapse-item>
          <n-collapse-item name="privacy">
            <template #header>
              <n-flex align="center" size="small">
                <n-icon color="#18a058"><ShieldCheckmarkOutline /></n-icon>
                <span>数据隐私</span>
              </n-flex>
            </template>
            <div class="collapse-inner">
              <n-alert type="success" :show-icon="false" class="mb-2">
                <strong>本地优先架构</strong><br />
                你的所有数据（库存、账目、图片）仅存储于设备的 SQLite
                数据库中，软件除自动更新检查外不进行任何网络通信。
              </n-alert>
              <p class="text-muted">
                我们承诺这个项目会尽可能地允许同好本地原子化部署，无需中心服务器，也不搜集任何隐私数据。
              </p>
            </div>
          </n-collapse-item>

          <n-collapse-item name="disclaimer">
            <template #header>
              <n-flex align="center" size="small">
                <n-icon color="#f0a020"><AlertCircleOutline /></n-icon>
                <span>免责声明</span>
              </n-flex>
            </template>
            <div class="collapse-inner">
              <p class="text-muted text-small">
                THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.
              </p>
              <p class="text-muted">
                本软件基于 MIT
                协议开源。开发者不对因使用产生的任何数据丢失或营业额偏差承担责任。基于该软件的二次开发须保留该声明。
              </p>
              <n-alert type="warning" :show-icon="false">
                <strong>⚠️ 重要提醒</strong
                >：请养成定时导出数据备份(包括制品信息和营业记录)的习惯，以防止意外发生。
              </n-alert>
            </div>
          </n-collapse-item>
        </n-collapse>
      </SectionCard>

      <!-- 故障排查：日志文件位置（只在桌面 / 手机应用里有） -->
      <SectionCard v-if="logDir" title="故障排查">
        <p class="log-desc">
          程序运行日志保存在下面的文件夹里（<code>booth.log</code>）。遇到闪退或功能异常时，
          把这个文件发给开发者能大大加快排查。
        </p>
        <n-flex align="center" :wrap="true">
          <code class="log-path">{{ logDir }}</code>
          <n-button size="small" @click="copyLogDir">复制路径</n-button>
        </n-flex>
      </SectionCard>

      <!-- 开发者与联系方式 -->
      <SectionCard title="开发者与联系方式">
        <n-flex vertical align="center" size="large">
          <div class="dev-info">
            <img src="/avatar.png" alt="Renko_1055" class="author-avatar" />
            <div class="text-center">
              <div class="author-name">Renko_1055</div>
              <div class="author-title">境界景观学会</div>
            </div>
          </div>

          <n-flex>
            <n-button
              secondary
              type="default"
              size="small"
              @click="
                copyLink('https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp', 'GitHub')
              "
            >
              <template #icon
                ><n-icon><LogoGithub /></n-icon
              ></template>
              GitHub
            </n-button>
            <n-button
              secondary
              type="info"
              size="small"
              @click="copyLink('https://abl.secret-sealing.club', '官网')"
            >
              <template #icon
                ><n-icon><GlobeOutline /></n-icon
              ></template>
              官网
            </n-button>
            <n-popover trigger="hover">
              <template #trigger>
                <n-button
                  secondary
                  type="primary"
                  size="small"
                  @click="copyLink('1074201740', 'QQ群号')"
                >
                  <template #icon
                    ><n-icon><ChatbubblesOutline /></n-icon
                  ></template>
                  加入用户反馈QQ群
                </n-button>
              </template>
              <span>群号：1074201740</span>
            </n-popover>
            <n-popover trigger="hover">
              <template #trigger>
                <n-button
                  secondary
                  type="success"
                  size="small"
                  @click="copyLink('contact@secret-sealing.club', '邮箱')"
                >
                  <template #icon
                    ><n-icon><MailOutline /></n-icon
                  ></template>
                  联系
                </n-button>
              </template>
              <span>contact@secret-sealing.club</span>
            </n-popover>
          </n-flex>

          <div class="copyright">© 2026 境界景观学会 | Designed for Doujin Circles</div>
        </n-flex>
      </SectionCard>

      <!-- 危险操作区 -->
      <SectionCard title="危险操作">
        <div class="danger-zone">
          <div class="danger-header">
            <n-icon :color="'var(--error-color)'"><TrashOutline /></n-icon> 危险操作
          </div>
          <p class="danger-desc">清空所有数据和图片文件，仅在重置系统时使用。</p>
          <n-button type="error" ghost size="small" @click="resetDatabase"> 重置数据库 </n-button>
        </div>
      </SectionCard>
    </n-space>

    <UpdateModal :show="showUpdateModal" @update:show="showUpdateModal = $event" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  NAlert,
  NAvatar,
  NBlockquote,
  NButton,
  NCard,
  NCollapse,
  NCollapseItem,
  NDivider,
  NFlex,
  NGrid,
  NGridItem,
  NIcon,
  NPopover,
  NSpace,
  NTag,
  NTimeline,
  NTimelineItem,
} from 'naive-ui'
import {
  AlertCircleOutline,
  ChatbubblesOutline,
  GiftOutline,
  GlobeOutline,
  LogoAndroid,
  LogoGithub,
  LogoTux,
  LogoWindows,
  MailOutline,
  PhonePortraitOutline,
  ShieldCheckmarkOutline,
  StorefrontOutline,
  TrashOutline,
  WifiOutline,
} from '@vicons/ionicons5'
import { SectionCard } from '@/components/ui'
import UpdateModal from '@/components/shared/UpdateModal.vue'
import { api, unwrap, errorMessage } from '@/api/client'
import { copyLink as copyLinkUtil } from '@/services/clipboard'
import { useFeedback } from '@/composables/useFeedback'

const fb = useFeedback()
const showUpdateModal = ref(false)

// ---- 故障排查：日志目录 ----
const logDir = ref('')
onMounted(async () => {
  if (window.__TAURI_INTERNALS__ === undefined) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    logDir.value = await invoke<string>('get_log_dir')
  } catch (e) {
    console.warn('[AboutSettings] get_log_dir failed', e)
  }
})

async function copyLogDir() {
  try {
    await copyLinkUtil(logDir.value)
    fb.success('已复制日志文件夹路径')
  } catch {
    fb.info(logDir.value)
  }
}

// 数据定义：将原来硬编码在模板里的内容提取出来，使模板更干净
const features = [
  {
    title: '多机联动',
    icon: PhonePortraitOutline,
    color: '#2080f0',
    desc: '只需一台主机，局域网内其他设备扫码同样可运行程序变成“点单机”，低成本实现多设备协同。',
  },
  {
    title: '离线优先',
    icon: WifiOutline,
    color: '#18a058',
    desc: '基于本地 SQLite 和局域网部署，无论漫展现场信号如何，记账与库存扣减永远流畅。',
  },
  {
    title: '自动化经营',
    icon: StorefrontOutline,
    color: '#f0a020',
    desc: '自动算总价、弹收款码、生成 Excel 报表。摊主只需无脑递货，可专注于和同好交流。',
  },
]

const credits = [
  { title: '💻 同人社团', desc: '该项目由同人社团 境界景观学会 开发和维护。' },
  { title: '👥 内测者', desc: '感谢所有参与该项目内测的摊主们，你们的反馈至关重要。' },
  { title: '🤖 AI 助手', desc: '感谢 Gemini 3.0 Pro 和 Github Copilot 辅助开发项目原型。' },
  { title: '🎮 支持者', desc: '感谢东方幻想指南的朋友们对该项目开发的支持。' },
]

// 统一使用共享 clipboard 工具
const copyLink = async (url: string, label: string) => {
  try {
    await copyLinkUtil(url)
    fb.success(`已复制${label}链接`)
  } catch (err) {
    console.error('复制失败:', err)
    fb.error(`复制${label}失败，请检查权限`)
  }
}

const resetDatabase = async () => {
  await fb.confirm({
    title: '⚠️ 危险操作',
    content: '此操作将不可逆地删除所有展会、商品、订单数据及图片文件。\n确定要完全重置系统吗？',
    positiveText: '确认重置',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      const stopLoading = fb.loading('正在重置...')
      try {
        const res = await unwrap(api.PUT('/admin/reset-database'))
        stopLoading()
        fb.success(res.message || '重置成功')
        setTimeout(() => {
          sessionStorage.clear()
          window.location.href = '/admin'
        }, 1500)
      } catch (err) {
        stopLoading()
        fb.error(errorMessage(err, '重置失败'))
      }
    },
  })
}
</script>

<style scoped>
.about-settings {
  scroll-margin-top: var(--space-xl);
}

.text-muted {
  color: var(--text-muted);
  line-height: 1.6;
}
.text-small {
  font-size: var(--font-sm);
}
.mb-2 {
  margin-bottom: var(--space-sm);
}

/* 头部 Header */
.header-section {
  text-align: center;
  padding: var(--space-lg) 0;
}
.logo {
  margin-bottom: var(--space-lg);
  box-shadow: var(--shadow-md);
}
.app-title {
  margin: 0 0 var(--space-sm);
  font-size: var(--font-2xl);
  font-weight: var(--weight-bold);
  letter-spacing: -0.5px;
}
.app-subtitle {
  font-size: var(--font-lg);
  color: var(--text-muted);
  margin: 0 0 var(--space-xl);
}
.update-row {
  margin-top: var(--space-lg);
}

/* 核心功能卡片 */
.feature-card {
  height: 100%;
  border-radius: var(--radius-md);
  transition: transform 0.2s;
}
.feature-card:hover {
  transform: translateY(-3px);
}
.feature-header {
  margin-bottom: var(--space-md);
}
.feature-title {
  font-weight: var(--weight-bold);
  font-size: var(--font-md);
  margin-left: var(--space-sm);
}
.feature-desc {
  color: var(--text-muted);
  font-size: var(--font-base);
  margin: 0;
  line-height: 1.5;
}

/* 致谢模块 */
.credit-item {
  background: color-mix(in srgb, var(--text-muted) 8%, transparent);
  padding: var(--space-md) var(--space-lg);
  border-radius: var(--radius-md);
  height: 100%;
}
.credit-title {
  font-weight: var(--weight-bold);
  font-size: var(--font-base);
  margin-bottom: var(--space-xs);
}
.credit-desc {
  font-size: var(--font-base);
  color: var(--text-muted);
}

/* 折叠面板内容 */
.collapse-inner {
  padding: var(--space-md) var(--space-xs);
  font-size: var(--font-base);
}

/* 开发者 */
.dev-info {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-sm);
}
.author-avatar {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  object-fit: cover;
  box-shadow: var(--shadow-md);
}
.author-name {
  font-weight: var(--weight-bold);
  font-size: var(--font-lg);
}
.author-title {
  font-size: var(--font-sm);
  color: var(--text-muted);
}
.copyright {
  font-size: var(--font-sm);
  color: var(--text-muted);
  font-family: monospace;
}

/* 危险操作区 */
.danger-zone {
  border: 1px dashed var(--error-color);
  background: var(--accent-color-light);
  border-radius: var(--radius-md);
  padding: var(--space-lg);
  width: 100%;
  max-width: 25rem;
  margin: 0 auto;
}
.danger-header {
  color: var(--error-color);
  font-weight: var(--weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-xs);
}
.danger-desc {
  font-size: var(--font-sm);
  color: var(--text-muted);
  margin: 0 0 var(--space-md);
}

.log-desc {
  color: var(--text-muted);
  margin: 0 0 var(--space-md);
}
.log-path {
  word-break: break-all;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--text-muted) 12%, transparent);
}

@media (--phone) {
  .app-title {
    font-size: var(--font-xl);
  }
  .logo {
    width: 80px !important;
    height: 80px !important;
  }
}
</style>
