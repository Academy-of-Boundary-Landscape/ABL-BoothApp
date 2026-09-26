<template>
  <SectionCard id="lan" title="局域网连接" collapsible v-model:collapsed="collapsed">
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
              确认两台设备连的是<strong>同一个 WiFi 名称</strong>（会场常有多个相近名字，别选错）
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
        <span class="hint">生成当前局域网的访问二维码，给点单平板和摊主手机扫</span>
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
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NAlert, NButton, NSpace } from 'naive-ui'
import QrcodeVue from 'qrcode.vue'
import { SectionCard } from '@/components/ui'
import { api, unwrap, errorMessage, type Schemas } from '@/api/client'
import { copyLink } from '@/services/clipboard'
import { useFeedback } from '@/composables/useFeedback'

const fb = useFeedback()

const collapsed = ref(false)
const isFetching = ref(false)
const fetchError = ref('')
const serverInfo = ref<Schemas['ServerInfo'] | null>(null)

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
</script>

<style scoped>
.hint {
  font-size: var(--font-sm);
  color: var(--text-muted);
}

.qr-actions {
  display: flex;
  align-items: center;
  gap: var(--space-lg);
  flex-wrap: wrap;
}

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
</style>
