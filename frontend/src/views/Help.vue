<template>
  <div class="tutorial-container">
    <n-card :bordered="false" class="main-card" size="large">
      
      <!-- 1. 头部 Header -->
      <div class="header-section">
        <n-icon size="64" color="var(--accent-color)"><BookOutline /></n-icon>
        <h1 class="page-title">使用教程</h1>
        <p class="page-subtitle">从零开始，快速掌握摊盒的核心功能</p>
      </div>

      <n-divider />

      <!-- 2. 主体教程轮播图 -->
      <section class="section">
        <div class="section-header">
          <n-icon size="24" color="var(--accent-color)"><PlayCircleOutline /></n-icon>
          <h2>核心流程演示</h2>
        </div>
        
        <n-carousel
          show-arrow
          dot-type="line"
          class="tutorial-carousel"
          draggable
        >
          <div v-for="(step, index) in tutorialSteps" :key="index" class="carousel-item">
            <div class="step-image-wrapper">
              <img :src="step.imgName" :alt="step.title" class="step-image" />
              <div class="step-badge">步骤 {{ index + 1 }}</div>
            </div>
            <div class="step-content">
              <h3>{{ step.title }}</h3>
              <p class="text-muted">{{ step.desc }}</p>
            </div>
          </div>
        </n-carousel>
      </section>

      <!-- 3. 快速上手 Grid -->
       <section class="section">
        <div class="section-header">
          <n-icon size="24" color="var(--success-color)"><RocketOutline /></n-icon>
          <h2>快速上手步骤</h2>
        </div>
        <n-grid x-gap="16" y-gap="16" cols="2 s:2 m:4" responsive="screen">
          <n-grid-item v-for="(item, idx) in quickSteps" :key="idx">
            <n-card class="quick-step-card" embedded :bordered="false">
              <!-- 使用 NFlex 和 NAvatar 实现完美居中对齐 -->
              <n-flex vertical align="center" justify="center" :size="12">
                <n-avatar
                  round
                  :size="44"
                  class="step-number-avatar"
                >
                  {{ idx + 1 }}
                </n-avatar>
                <div class="quick-step-text">{{ item }}</div>
              </n-flex>
            </n-card>
          </n-grid-item>
        </n-grid>
      </section>

      <n-divider />

      <!-- 4. 离散 QA (带搜索) -->
       <section class="section qa-section">
        <div class="section-header qa-header-flex">
          <div class="flex-center">
            <n-icon size="24" color="var(--warning-color)"><HelpCircleOutline /></n-icon>
            <h2>常见问题解答</h2>
          </div>
          <n-flex align="center">
             <n-button size="tiny" quaternary @click="toggleAllCategories(true)">全部展开</n-button>
             <n-button size="tiny" quaternary @click="toggleAllCategories(false)">一键收起</n-button>
             <n-input v-model:value="searchQuery" placeholder="搜索关键词..." clearable class="qa-search">
              <template #prefix><n-icon><SearchOutline /></n-icon></template>
            </n-input>
          </n-flex>
        </div>

        <div v-if="filteredQA.length > 0">
          <!-- 分类折叠面板 -->
          <n-collapse :expanded-names="expandedCategories" @update:expanded-names="handleCategoryChange">
            <n-collapse-item v-for="category in filteredQA" :key="category.category" :name="category.category">
              <template #header>
                <n-flex align="center" size="small">
                  <n-icon :component="category.icon" color="var(--accent-color)" />
                  <span class="category-title-text">{{ category.category }}</span>
                  <n-badge :value="category.items.length" type="info" :inverted="true" />
                </n-flex>
              </template>
              
              <!-- 具体的 QA 列表 -->
              <div class="category-inner-content">
                <n-collapse arrow-placement="right" plain>
                  <n-collapse-item 
                    v-for="(item, index) in category.items" 
                    :key="index"
                    :title="item.q" 
                    :name="index"
                  >
                    <div class="qa-answer" v-html="item.a"></div>
                  </n-collapse-item>
                </n-collapse>
              </div>
            </n-collapse-item>
          </n-collapse>
        </div>
        
        <n-empty v-else description="没有找到相关问题" class="mt-4" />
      </section>

      <!-- 5. 底部反馈 -->
      <section class="footer-section">
        <n-divider />
        <p class="text-muted text-small">教程未能解决您的问题？</p>
        <n-flex justify="center" class="mt-2">
          <n-popover trigger="hover">
            <template #trigger>
              <n-button secondary type="primary" size="small" @click="copyLink('1074201740', '用户交流群链接')">
                <template #icon><n-icon><ChatbubblesOutline /></n-icon></template>
                加入用户交流群
              </n-button>
            </template>
            <span>用户交流qq群链接已复制到剪贴板</span>
          </n-popover>
          <n-button secondary type="info" size="small" @click="copyLink('contact@secret-sealing.club', '邮箱')">
            <template #icon><n-icon><MailOutline /></n-icon></template>
            发送反馈邮件
          </n-button>
        </n-flex>
        <div class="copyright mt-4">
          © 2026 境界景观学会 | Documentation
        </div>
      </section>

    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { 
  NCard, NDivider, NGrid, NGridItem, NIcon, NButton, NFlex, 
  NCollapse, NCollapseItem, NCarousel, NInput, NEmpty, NPopover, NBadge, useMessage
} from 'naive-ui'
import { copyLink as copyLinkUtil } from '@/services/clipboard'
import { 
  BookOutline, PlayCircleOutline, RocketOutline, HelpCircleOutline, 
  SearchOutline, ChatbubblesOutline, MailOutline, WifiOutline, 
  WalletOutline, ShieldCheckmarkOutline, SettingsOutline, AlertCircleOutline, 
  ImageOutline, HardwareChipOutline, LogoGithub
} from '@vicons/ionicons5'

const message = useMessage()

// QA 分类展开状态
const expandedCategories = ref([])

// 轮播图数据
const tutorialSteps = [
  {
    shortTitle: '安装录入',
    title: '第一步：准备工作',
    desc: '在一台设备上下载并安装摊盒 App，将社团的制品图、名称及价格完整录入系统。',
    imgName: '/help/p1.png'
  },
  {
    shortTitle: '展会备货',
    title: '第二步：配置展会',
    desc: '在漫展开始前，于系统内创建新展会，并根据实际备货情况填写各制品的库存数量。',
    imgName: '/help/p2.png'
  },
  {
    shortTitle: '局域网连接',
    title: '第三步：组建局域网',
    desc: '漫展现场，用平板开启热点（无网络也可），手机打开浏览器，扫描平板上的二维码，实现多端秒连。',
    imgName: '/help/p3.png'
  },
  {
    shortTitle: '现场出摊',
    title: '第四步：自助点单',
    desc: '平板作为“顾客视图”摆在摊位前。顾客自助点单后，摊主手机端会立即收到配货提示。',
    imgName: '/help/p4.png'
  },
  {
    shortTitle: '一键总结',
    title: '第五步：收摊结算',
    desc: '漫展结束后，摊主可以在统计页面轻松地一键导出全天的销售总结与财务报。',
    imgName: '/help/p5.png'
  }
]
// 快速上手文字
const quickSteps = ['录入制品', '建立展会', '扫码配对', '开始出摊']

// QA 数据 - 按分类组织
const searchQuery = ref('')
const qaList = [
  {
    category: '网络连接',
    icon: WifiOutline,
    items: [
      { 
        q: '手机扫码后显示“连接超时”或网页无法打开？', 
        a: '这是最常见的问题，请按顺序排查：<br/>1. <strong>检查热点</strong>：确保主机（电脑/平板）和扫码的手机连的是同一个 Wi-Fi/热点。<br/>2. <strong>防火墙拦截</strong>：Windows 主机请检查防火墙是否允许了“摊盒”通过，或尝试暂时关闭防火墙。<br/>3. <strong>IP变动</strong>：如果重启过热点，IP 地址可能会变，请在主机端点击“重新生成二维码”。' 
      },
      { 
        q: '没有网络/信号差能用吗？', 
        a: '<strong>完全可以。</strong> 摊盒是“离线优先”的设计。推荐用一台设备开启<strong>手机热点</strong>组建局域网，其他设备连接它即可，不需要连接互联网，也不消耗流量。' 
      },
      { 
        q: '我是校园网/公共 Wi-Fi，设备连不上？', 
        a: '公共网络通常开启了“AP 隔离”功能，禁止设备间互相访问。<strong>请务必使用热点</strong>来组建网络，这是漫展现场最稳妥的方案。' 
      },
      { 
        q: '中途断网了，数据会丢吗？', 
        a: '<strong>不会。</strong> 只要主机端（App端）没有关闭，数据就一直存在。重新连接网络后，手机端刷新页面即可恢复之前的状态。' 
      }
    ]
  },
  {
    category: '现场运营',
    icon: WalletOutline,
    items: [
      { 
        q: '怎么设置收款码？支持多渠道吗？', 
        a: '在“展会管理”点击“编辑”，可以上传图片。如果您同时支持微信/支付宝，可以将两个二维码<strong>拼成一张长图</strong>上传。' 
      },
      { 
        q: '顾客付款后，系统会自动完成订单吗？', 
        a: '<strong>不会。</strong> 为了保护隐私及规避金融风险，摊盒不接入支付接口。您听到“支付宝/微信到账”的提示后，需在摊主手机上手动点击“完成订单”来扣减库存。' 
      },
      { 
        q: '有新订单时会有声音提示吗？', 
        a: '<strong>有的。</strong> 摊主手机端（连接端）在收到新下单请求时会播放提示音。请确保您的手机<strong>未处于静音模式</strong>，并调大媒体音量。' 
      },
      { 
        q: '可以多人/多设备同时摆摊吗？', 
        a: '支持！只要连入同一个热点，您可以放置 2 台平板作为“点单机”，并让 3 位摊主都拿着手机作为“配货机”，所有数据实时同步。' 
      }
    ]
  },
  {
    category: '突发状况',
    icon: AlertCircleOutline,
    items: [
      { 
        q: '顾客下错单/我想取消订单怎么办？', 
        a: '在摊主配货页面，不要点“完成”，直接点击红色的<strong>“取消”</strong>按钮。该订单会被废弃，锁定的库存会自动返还。' 
      },
      { 
        q: '手滑误点了“完成”，想反悔怎么办？', 
        a: '别慌，请到主机的“订单管理”页面，找到那笔误操作的订单，将其状态修改为<strong>“已取消”</strong>。系统会重新把库存加回去，并修正销售额统计。' 
      },
      { 
        q: '主机设备突然没电/死机了怎么办？', 
        a: '摊盒采用 SQLite 实时落盘存储。重启设备和软件后，所有的商品信息、历史订单和库存数据都会<strong>自动恢复</strong>到死机前的那一刻。但为了体验方便，请准备好<strong>备用电源</strong>，以防万一。' 
      }
    ]
  },
  {
    category: '图片显示',
    icon: ImageOutline,
    items: [
      { 
        q: '商品图片无法显示或加载很慢？', 
        a: '1. 请检查图片文件名是否包含特殊符号（建议使用纯数字或英文命名）。<br/>2. 局域网传输带宽有限，建议将单张图片压缩在 <strong>500KB 以内</strong>，不要直接上传 10MB 的高清原图。' 
      },
      { 
        q: '我可以自定义界面样式吗？', 
        a: '目前为了方便，只支持自定义亮暗与主题颜色，更细致的自定义主题功能将在未来开放。' 
      }
    ]
  },
  {
    category: '数据安全与迁移',
    icon: ShieldCheckmarkOutline,
    items: [
      { 
        q: '我的数据存储在哪里？安全吗？', 
        a: '所有数据（图片、账本、库存）均存储在您<strong>主机设备的本地数据库</strong>中。我们没有任何云端服务器，您的营业额数据除了您自己没人知道。' 
      },
      { 
        q: '换了新设备，怎么迁移数据？', 
        a: '在“全局商品库”页面，点击“导出数据包”，会将所有商品信息和图片打包下载。将该文件发送到新设备，在设置页选择“导入数据”即可。' 
      },
      { 
        q: '导出的 Excel 包含哪些内容？', 
        a: '包含展会概览以及关键的<strong>商品销量统计表</strong>。您可以直接把这个表发给社团主催交差。' 
      }
    ]
  },
  {
    category: '高级技巧',
    icon: SettingsOutline,
    items: [
      { 
        q: '如何设置打折/满减优惠？', 
        a: '为了保持记账逻辑简单，系统暂无复杂的优惠券功能。<br/><strong>小技巧：</strong>您可以创建一个名为“优惠/抹零”的商品，价格设为 <strong>-5</strong> 元。结算时加入这个商品，总价就会自动减去 5 元。' 
      },
      { 
        q: '支持“捆绑销售”或“套装”吗？', 
        a: '建议直接录入一个名为“XX套装”的新商品，并设置好打包价。这样在统计销量时也能清楚地知道卖出了多少个套装。' 
      },
      { 
        q: '我该如何处理售罄的商品？', 
        a: '在库存不足和售罄时，顾客端会自动将该商品置灰并禁止下单。您无需手动隐藏或下架商品。' 
      }
    ]
  },
  {
    category: '硬件建议',
    icon: HardwareChipOutline,
    items: [
      { 
        q: '我可以在手机上运行主机端吗？', 
        a: '技术上可以，但<strong>强烈不推荐</strong>。主机需要长时间运行服务，手机容易因锁屏、后台杀进程或发热导致服务中断。<strong>推荐使用笔记本电脑或平板作为主机。</strong>' 
      },
      { 
        q: '对设备性能有要求吗？', 
        a: '要求很低。任何能运行最新版 Chrome/Edge 的 Windows 电脑或 Android 8.0+ 的平板均可流畅运行。老旧设备作为“顾客点单机”也是完美的废物利用方案。' 
      },
      { 
        q: '需要一直亮屏吗？', 
        a: '主机端建议保持亮屏或在电源设置中设置为“不休眠”。顾客端和平板建议在浏览器中设置“屏幕常亮”，以免顾客点单时屏幕突然熄灭。' 
      }
    ]
  },
  {
    category: '社区与开源',
    icon: LogoGithub,
    items: [
      { 
        q: '去哪里下载最新版本？', 
        a: '请认准唯一的官方渠道：<strong>GitHub Releases</strong> 页面。我们会在那里第一时间发布新版本。任何要求付费下载或“代部署收费”的渠道均为诈骗，请勿上当。' 
      },
      {
        q: '我能自动更新吗？', 
        a: '请点击侧边栏的“检查更新”按钮，系统会自动检测最新版本并引导您下载安装包。当前版本不支持一键自动更新，需手动下载安装包进行覆盖安装。'
      },
      { 
        q: '遇到 Bug 或有建议去哪里反馈？', 
        a: '如果是程序报错，强烈建议您在 GitHub 仓库提交 <strong>Issue</strong>，这样能方便开发者追踪修复。如果是使用疑惑或单纯想闲聊，欢迎点击下方的按钮加入<strong>用户交流群</strong>。' 
      },
      { 
        q: '我是开发者，可以为项目贡献代码吗？', 
        a: '<strong>非常欢迎！</strong> 这是一个开源项目，我们期待您提交 <strong>Pull Request</strong>。无论是修复一个小 Bug，还是开发一个全新的功能模块，您的贡献都将帮助到成千上万的摊主。' 
      },
      { 
        q: '我能修改软件并重新发布（甚至售卖）吗？', 
        a: '本项目遵循 <strong>MIT 协议</strong>，您确实拥有修改和分发的自由。但作为一款旨在“降低同人出摊门槛”的免费工具，我们<strong>强烈不推荐</strong>将其包装为商业软件进行售卖。请保留原作者信息，尊重开源精神。' 
      }
    ]
  }
]
// 搜索过滤逻辑 - 支持分类结构
const filteredQA = computed(() => {
  const query = searchQuery.value.toLowerCase().trim()
  if (!query) return qaList
  
  // 过滤每个分类下的问题
  return qaList
    .map(category => ({
      ...category,
      items: category.items.filter(item =>
        item.q.toLowerCase().includes(query) ||
        item.a.toLowerCase().includes(query)
      )
    }))
    .filter(category => category.items.length > 0) // 只保留有匹配项的分类
})
// 分类展开收起逻辑
const handleCategoryChange = (names: string[]) => {
  expandedCategories.value = names
}

const toggleAllCategories = (expand: boolean) => {
  if (expand) {
    expandedCategories.value = filteredQA.value.map(c => c.category)
  } else {
    expandedCategories.value = []
  }
}
// 统一使用共享 clipboard 工具
const copyLink = async (url: string, label: string) => {
  try {
    await copyLinkUtil(url)
    message.success(`已复制${label}`)
  } catch (err) {
    console.error('复制失败:', err)
    message.error(`复制${label}失败，请检查权限`)
  }
}
</script>

<style scoped>
.tutorial-container {
  max-width: 800px;
  margin: 0 auto;
  padding: 24px 16px;
  --text-primary: var(--primary-text-color);
  --text-secondary: var(--secondary-text-color);
  --bg-subtle: rgba(128, 128, 128, 0.08);
}

/* 通用排版工具类 */
.text-muted { color: var(--text-muted); line-height: 1.6; }
.text-small { font-size: 0.85rem; }
.mb-2 { margin-bottom: 8px; }
.mt-2 { margin-top: 16px; }
.mt-4 { margin-top: 32px; }

/* Header */
.header-section { 
  text-align: center; 
  padding: 20px 0; 
}
.page-title { 
  margin: 8px 0 4px; 
  font-size: 2rem; 
  font-weight: 800;
  letter-spacing: -0.5px;
  color: var(--primary-text-color);
}
.page-subtitle { 
  font-size: 1.1rem; 
  color: var(--secondary-text-color); 
  margin-bottom: 20px;
  line-height: 1.6;
}

/* Section General */
.section { margin-bottom: 40px; }
.section-header { 
  display: flex; 
  align-items: center; 
  gap: 10px; 
  margin-bottom: 20px;
}
.section-header h2 { 
  margin: 0; 
  font-size: 1.25rem; 
  font-weight: 600;
  color: var(--primary-text-color);
}

/* Carousel Tutorial */
.tutorial-carousel {
  border-radius: 12px;
  overflow: hidden;
  background: var(--bg-subtle);
  max-height: 500px;
}

.carousel-item {
  display: flex;
  flex-direction: column;
  max-height: 500px;
}

.step-image-wrapper {
  position: relative;
  width: 100%;
  aspect-ratio: 16 / 9;
  background: var(--tertiary-text-color);
  max-height: 350px;
  overflow: hidden;
}

.step-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.step-badge {
  position: absolute;
  top: 12px;
  left: 12px;
  background: var(--accent-color);
  color: var(--text-white);
  padding: 4px 12px;
  border-radius: 20px;
  font-weight: bold;
  font-size: 0.8rem;
}

.step-content {
  padding: 20px;
  text-align: center;
  overflow-y: auto;
  max-height: 150px;
}

.step-content h3 { 
  margin: 0 0 8px; 
  font-size: 1.2rem;
  color: var(--primary-text-color);
}

/* 修复轮播图按钮在浅色主题下的可见性 */
:deep(.n-carousel__arrow) {
  background-color: rgba(0, 0, 0, 0.3) !important;
  color: var(--primary-text-color) !important;
  border-radius: 4px;
}

:deep(.n-carousel__arrow:hover) {
  background-color: rgba(0, 0, 0, 0.5) !important;
}

:deep(.n-carousel__dots) {
  background: rgba(0, 0, 0, 0.05);
  padding: 8px;
  border-radius: 8px;
}

/* Quick Steps Card */
.quick-step-card {
  background: var(--bg-subtle);
  padding: 16px;
  border-radius: 12px;
  text-align: center;
  transition: all 0.3s;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.quick-step-card:hover {
  background: rgba(128, 128, 128, 0.12);
  transform: translateY(-2px);
}

.step-number-avatar {
  background-color: var(--bg-subtle) !important;
  color: var(--accent-color) !important;
  font-weight: 700;
  font-size: 1.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.quick-step-text { 
  font-weight: 600; 
  font-size: 0.95rem;
  color: var(--primary-text-color);
}

/* QA Section */
.qa-header-flex {
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 16px;
}

.flex-center { 
  display: flex; 
  align-items: center; 
  gap: 10px; 
}

.qa-search { 
  width: 240px; 
}

/* QA 分类样式 */
.qa-category {
  margin-bottom: 32px;
}

.category-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 2px solid var(--accent-color);
}

.category-title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--primary-text-color);
}

.category-count {
  font-size: 0.85rem;
  color: var(--text-muted);
  background: var(--bg-subtle);
  padding: 4px 12px;
  border-radius: 12px;
}

.qa-answer { 
  line-height: 1.6; 
  padding: 4px 0;
  color: var(--text-muted);
}

/* Footer */
.footer-section { 
  text-align: center; 
  padding-bottom: 20px; 
}

.copyright { 
  font-size: 0.8rem; 
  color: var(--text-muted); 
  font-family: monospace;
}

/* Responsive */
@media (max-width: 600px) {
  .qa-search { 
    width: 100%; 
  }
  .page-title { 
    font-size: 1.6rem; 
  }
  .section-header h2 { 
    font-size: 1.1rem; 
  }
  .qa-header-flex {
    flex-direction: column;
    align-items: flex-start;
  }
  .flex-center {
    width: 100%;
  }
}
</style>