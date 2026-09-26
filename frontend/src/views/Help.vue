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
        <div class="section-heading">
          <n-icon size="24" color="var(--accent-color)"><PlayCircleOutline /></n-icon>
          <h2>核心流程演示</h2>
        </div>

        <n-carousel show-arrow dot-type="line" class="tutorial-carousel" draggable>
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
        <div class="section-heading">
          <n-icon size="24" color="var(--success-color)"><RocketOutline /></n-icon>
          <h2>快速上手步骤</h2>
        </div>
        <n-grid x-gap="16" y-gap="16" cols="2 s:2 m:4" responsive="screen">
          <n-grid-item v-for="(item, idx) in quickSteps" :key="idx">
            <n-card class="quick-step-card" embedded :bordered="false">
              <!-- 使用 NFlex 和 NAvatar 实现完美居中对齐 -->
              <n-flex vertical align="center" justify="center" :size="12">
                <n-avatar round :size="44" class="step-number-avatar">
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
        <div class="section-heading qa-header-flex">
          <div class="flex-center">
            <n-icon size="24" color="var(--warning-color)"><HelpCircleOutline /></n-icon>
            <h2>常见问题解答</h2>
          </div>
          <n-flex align="center">
            <n-button size="tiny" quaternary @click="toggleAllCategories(true)">全部展开</n-button>
            <n-button size="tiny" quaternary @click="toggleAllCategories(false)">一键收起</n-button>
            <n-input
              v-model:value="searchQuery"
              placeholder="搜索关键词..."
              clearable
              class="qa-search"
            >
              <template #prefix
                ><n-icon><SearchOutline /></n-icon
              ></template>
            </n-input>
          </n-flex>
        </div>

        <div v-if="filteredQA.length > 0">
          <!-- 分类折叠面板 -->
          <n-collapse
            :expanded-names="expandedCategories"
            @update:expanded-names="handleCategoryChange"
          >
            <n-collapse-item
              v-for="category in filteredQA"
              :key="category.category"
              :name="category.category"
            >
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
              <n-button
                secondary
                type="primary"
                size="small"
                @click="copyLink('1074201740', '用户交流群链接')"
              >
                <template #icon
                  ><n-icon><ChatbubblesOutline /></n-icon
                ></template>
                加入用户交流群
              </n-button>
            </template>
            <span>用户交流qq群链接已复制到剪贴板</span>
          </n-popover>
          <n-button
            secondary
            type="info"
            size="small"
            @click="copyLink('contact@secret-sealing.club', '邮箱')"
          >
            <template #icon
              ><n-icon><MailOutline /></n-icon
            ></template>
            发送反馈邮件
          </n-button>
        </n-flex>
        <div class="copyright mt-4">© 2026 境界景观学会 | Documentation</div>
      </section>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  NCard,
  NDivider,
  NGrid,
  NGridItem,
  NIcon,
  NButton,
  NFlex,
  NCollapse,
  NCollapseItem,
  NCarousel,
  NInput,
  NEmpty,
  NPopover,
  NBadge,
} from 'naive-ui'
import { copyLink as copyLinkUtil } from '@/services/clipboard'
import { useFeedback } from '@/composables/useFeedback'
import {
  BookOutline,
  PlayCircleOutline,
  RocketOutline,
  HelpCircleOutline,
  SearchOutline,
  ChatbubblesOutline,
  MailOutline,
  WifiOutline,
  WalletOutline,
  ShieldCheckmarkOutline,
  SettingsOutline,
  AlertCircleOutline,
  ImageOutline,
  HardwareChipOutline,
  LogoGithub,
} from '@vicons/ionicons5'

const fb = useFeedback()

// QA 分类展开状态
const expandedCategories = ref<string[]>([])

// 轮播图数据
const tutorialSteps = [
  {
    shortTitle: '安装录入',
    title: '第一步：准备工作',
    desc: '在一台设备上安装摊盒 App，到「商品库」录入制品的编号、名称、价格和图片。帮别的社团代卖的，先在「社团」里建好对方，再把商品归到它名下。',
    imgName: '/help/p1.png',
  },
  {
    shortTitle: '展会备货',
    title: '第二步：配置展会',
    desc: '新建展会并进入展会工作台：在「展前 · 商品」从商品库选或从上一场导入，填好库存；要优惠就在「展前 · 套装」配套装。准备好后点「开始展会」。',
    imgName: '/help/p2.png',
  },
  {
    shortTitle: '局域网连接',
    title: '第三步：组建局域网',
    desc: '漫展现场用一台手机开热点（不需要流量），所有设备连上。主机在「设置 → 局域网连接」获取二维码：平板扫「顾客入口」，摊主手机扫「摊主入口」。',
    imgName: '/help/p3.png',
  },
  {
    shortTitle: '现场出摊',
    title: '第四步：自助点单',
    desc: '平板摆在摊位前当点单页，顾客自助加购（也能扫码、拍照找商品）。摊主手机响起后配货，确认到账再点「完成配货」→ 选渠道 →「确认收款」。',
    imgName: '/help/p4.png',
  },
  {
    shortTitle: '一键总结',
    title: '第五步：收摊结算',
    desc: '散场时在摊主端「收摊」tab 走完 清点订单 → 盘点 → 带回 → 结算 四步，得到结算单：每个社团该分多少一目了然。销售汇总和结算单都能导出 Excel。',
    imgName: '/help/p5.png',
  },
]
// 快速上手文字
const quickSteps = ['录入制品', '建展会 · 上架', '热点 · 扫码组网', '出摊 · 收摊结算']

// QA 数据 - 按分类组织
const searchQuery = ref('')
const qaList = [
  {
    category: '网络连接',
    icon: WifiOutline,
    items: [
      {
        q: '手机扫码后显示“连接超时”或网页无法打开？',
        a: '这是最常见的问题，请按顺序排查：<br/>1. <strong>检查热点</strong>：确保主机（电脑/平板）和扫码的手机连的是同一个 Wi-Fi/热点。<br/>2. <strong>防火墙拦截</strong>：Windows 主机请检查防火墙是否允许了“摊盒”通过，或尝试暂时关闭防火墙。<br/>3. <strong>IP变动</strong>：如果重启过热点，IP 地址可能会变，请在主机「设置 → 局域网连接」重新点「获取局域网二维码」。<br/>4. <strong>证书警告</strong>：第一次打开会提示「连接不是私密连接」，点「高级 → 继续访问」即可。',
      },
      {
        q: '没有网络/信号差能用吗？',
        a: '<strong>完全可以。</strong> 摊盒是“离线优先”的设计。推荐用一台设备开启<strong>手机热点</strong>组建局域网，其他设备连接它即可，不需要连接互联网，也不消耗流量。',
      },
      {
        q: '我是校园网/公共 Wi-Fi，设备连不上？',
        a: '公共网络通常开启了“AP 隔离”功能，禁止设备间互相访问。<strong>请务必使用热点</strong>来组建网络，这是漫展现场最稳妥的方案。',
      },
      {
        q: '中途断网了，数据会丢吗？',
        a: '<strong>不会。</strong> 只要主机端（App端）没有关闭，数据就一直存在。重新连接网络后，手机端刷新页面即可恢复之前的状态。',
      },
    ],
  },
  {
    category: '现场运营',
    icon: WalletOutline,
    items: [
      {
        q: '怎么设置收款码？支持多渠道吗？',
        a: '新建展会或在展会工作台点「编辑」，可以<strong>分别上传微信和支付宝</strong>两张收款码。顾客结算页会自动并排显示两个码，顾客用哪个 App 扫哪个即可。只上传一张也行，这时页面单码居中显示。',
      },
      {
        q: '顾客付款后，系统会自动完成订单吗？',
        a: '<strong>不会。</strong> 为了保护隐私及规避金融风险，摊盒不接入支付接口。听到“支付宝/微信到账”的提示后，在摊主端这张订单上点「<strong>完成配货</strong>」，选好收款渠道，再点「<strong>确认收款</strong>」。<br/>这一步只是<strong>记账</strong>，系统没有能力核对钱是否真的到了。',
      },
      {
        q: '有新订单时会有声音提示吗？',
        a: '<strong>有的。</strong> 摊主端收到新订单时会播放提示音，切到「库存」「收摊」tab 也照常提醒。请确保手机<strong>未处于静音模式</strong>、调大媒体音量，并在页面上<strong>先点一下</strong>（浏览器要求页面被点过才能出声）。',
      },
      {
        q: '可以多人/多设备同时摆摊吗？',
        a: '支持！只要连入同一个热点，您可以放置 2 台平板作为“点单机”，并让 3 位摊主都拿着手机作为“配货机”，所有数据实时同步。',
      },
      {
        q: '摊主可以帮顾客点单吗？',
        a: '可以。摊主端页头点「<strong>去点单</strong>」进入点单页替顾客下单，点完点「<strong>回摊主端</strong>」回来收款。商品有条码的话，点单页的「扫码」或扫码枪可以更快加购。',
      },
      {
        q: '怎么用扫码加购？',
        a: '点单页工具栏点「<strong>▦ 扫码</strong>」，把条码对准取景框，扫中自动加入购物车，不用按快门。<br/>有商业条码的商品（ISBN / JAN），在商品库填上「商业条码」；没有的，扫码时按<strong>商品编号</strong>匹配，把编号打印成条码贴上即可。<br/>USB / 蓝牙扫码枪插上即用（Windows 上请切到英文输入法）。局域网浏览器里用摄像头需要走 https 地址。',
      },
    ],
  },
  {
    category: '收摊与结算',
    icon: WalletOutline,
    items: [
      {
        q: '赠送和报废怎么登记？',
        a: '顾客没买、货却不在摊上了，分两种：<br/><strong>赠送</strong>——你主动送出去的（亲友拿样、随机塞的小礼物）。在<strong>摊主端</strong>点「登记赠送/报废」，选商品和数量；如果这笔算你自掏，打开「这笔我自掏（按原价补给货主）」，结算时这笔钱会算到你头上，否则货主自己承担。<br/><strong>报废</strong>——损坏、脏污、缺件等不能再卖的，切到「报废」页登记，只动库存、不动钱。<br/>两者都能在列表里<strong>撤销</strong>（货回到现场仓），也都会出现在结算单的「货主明细」里，但不会进销售统计。',
      },
      {
        q: '卖出去的货顾客要退怎么办？',
        a: '在<strong>摊主端 → 订单 → 已完成</strong>列表里找到那笔订单，点「退货」。弹窗按<strong>订单行</strong>逐行列出（同一商品因为套装可能拆成多行），退哪一行、退几件由你决定；每行可选退回<strong>现场仓</strong>（还能再卖）或<strong>损耗</strong>（已损坏）。退款渠道默认与收款一致（现金收就现金退，否则收摊清点会对不上），退款金额默认按所选各行的实付合计，不能超过顾客实付。已退完的行会标「已退完」。<strong>退货没有撤销</strong>，退错了请再开一张反向的单或走结算调整。',
      },
      {
        q: '展会结束时怎么收摊？',
        a: '在<strong>摊主端</strong>切到「收摊」tab，跟着向导走四步：<strong>清点订单</strong> → <strong>盘点</strong>（可以跳过）→ <strong>带回</strong> → <strong>结算</strong>。第一步要<strong>先把 pending 订单（还没确认的单）逐单完成或取消</strong>，只要还有 pending 订单，后面三步都会被拒绝。盘点是把每个品种实际剩多少件数一遍，和账面一比，差额记成「盘点差异」；没盘点的话结算单会明确标注「未盘点，剩余数为账面推算」。带回之后每个商品的现场仓余额清零。结算会把展会冻结，「收摊」tab 原地变成这场的结算单——之后<strong>不能再下单或改单</strong>，但<strong>垫付、结算调整、收摊清点</strong>这三件事仍然可以回家补。',
      },
      {
        q: '帮别的社团代卖（寄售），怎么分开算账？',
        a: '先在侧栏「<strong>社团</strong>」里新建对方社团，再到「商品库」把他们的商品的「<strong>所属社团</strong>」改成对方（只影响之后上架的货）。之后卖出的钱会自动记在对方名下，结算单按社团分块，最后一行就是「<strong>我应转给 XX</strong>」。<br/>套装不能混搭不同社团的商品；收款时抹零、让价的差额全部算在本社团头上，不会让代卖社团少分钱。',
      },
      {
        q: '结算单里有什么？和销售统计的 Excel 有什么区别？',
        a: '<strong>销售统计</strong>的 Excel 回答「<strong>卖了什么</strong>」——各商品的销量和销售额。<strong>结算单</strong>（展会工作台 → 收摊 · 结算 → 导出 Excel；摊主端「收摊」tab 也能导出）回答「<strong>账怎么算</strong>」，有四个 sheet：<br/>• <strong>结算汇总</strong>：每个货主该分多少、「我应转给」多少，以及各收款渠道的账面 / 实际 / 差额；<br/>• <strong>货主明细</strong>：按货主 × 商品列出数量和原价 / 折让 / 净额；<br/>• <strong>账本流水</strong>：每条记账的时间、摘要和货腿 / 钱腿；<br/>• <strong>订单明细</strong>：每张订单每一行的货主应得、顾客实付和已退件数。<br/>寄售分账看结算单。注意「我应转给」只是账本算出来的应转数，<strong>系统没有能力核对任何一笔到账</strong>。',
      },
    ],
  },
  {
    category: '突发状况',
    icon: AlertCircleOutline,
    items: [
      {
        q: '顾客下错单/我想取消订单怎么办？',
        a: '在摊主端「订单 → 待处理」里，不要点「完成配货」，直接点这张订单的<strong>「取消」</strong>。该订单会被废弃，预留的库存自动返还。',
      },
      {
        q: '手滑误点了“完成”，想反悔怎么办？',
        a: '别慌：在管理后台进入这场展会的「<strong>现场 · 订单</strong>」，找到那笔订单，在「操作」里选<strong>「设为已取消」</strong>。系统会把货和钱两笔账一起冲回去，库存加回、销售额修正。<br/>已取消的订单不能再改状态。如果顾客只是退了部分商品，请用摊主端的「退货」，不要取消整单。',
      },
      {
        q: '主机设备突然没电/死机了怎么办？',
        a: '摊盒采用 SQLite 实时落盘存储。重启设备和软件后，所有的商品信息、历史订单和库存数据都会<strong>自动恢复</strong>到死机前的那一刻。但为了体验方便，请准备好<strong>备用电源</strong>，以防万一。',
      },
    ],
  },
  {
    category: '图片显示',
    icon: ImageOutline,
    items: [
      {
        q: '商品图片无法显示或加载很慢？',
        a: '1. 请检查图片文件名是否包含特殊符号（建议使用纯数字或英文命名）。<br/>2. 局域网传输带宽有限，建议将单张图片压缩在 <strong>500KB 以内</strong>，不要直接上传 10MB 的高清原图。',
      },
      {
        q: '我可以自定义界面样式吗？',
        a: '目前为了方便，只支持自定义亮暗与主题颜色，更细致的自定义主题功能将在未来开放。',
      },
    ],
  },
  {
    category: '数据安全与迁移',
    icon: ShieldCheckmarkOutline,
    items: [
      {
        q: '我的数据存储在哪里？安全吗？',
        a: '所有数据（图片、账本、库存）均存储在您<strong>主机设备的本地数据库</strong>中。我们没有任何云端服务器，您的营业额数据除了您自己没人知道。',
      },
      {
        q: '换了新设备，怎么迁移数据？',
        a: '在「商品库」页底部的「商品数据包」面板点「导出 .boothpack」，所有商品信息、社团归属和图片会打包下载。把文件发到新设备，在同一个面板点「导入 .boothpack」即可。<br/><strong>注意：</strong>.boothpack 只含商品库，不含展会、订单和账本。',
      },
      {
        q: '导出的 Excel 包含哪些内容？',
        a: '有两种：<br/>• <strong>销售汇总</strong>（现场 · 统计 → 下载 Excel 报告）：各商品卖了多少、卖了多少钱，适合当成绩单；<br/>• <strong>结算单</strong>（收摊 · 结算 → 导出 Excel）：每个社团该分多少、我应转给谁多少、各收款渠道的账面和实际。寄售分账看这一份。',
      },
      {
        q: '升级到 v1.2 后，旧的展会和订单去哪了？',
        a: 'v1.2 换了新账本，旧版的展会和订单没有迁移进来，但<strong>没有删除</strong>：它们原样保留在 <code>sale_system.db.v1-backup</code> 里。在「<strong>设置 → 历史数据（v1）</strong>」点「导出旧数据为 Excel」即可取出。商品库（含图片和识别数据）是自动带过来的。',
      },
    ],
  },
  {
    category: 'AI 拍照识别',
    icon: HardwareChipOutline,
    items: [
      {
        q: '拍照识别是什么？怎么用？',
        a: '拍照识别让顾客对准商品拍一张照片，系统自动识别是哪件商品并加入购物车，省去在列表里翻找的时间。<br/>使用前需要在 <strong>设置 → AI 视觉识别</strong> 面板确认模型已安装、索引已构建。',
      },
      {
        q: 'AI 识别的基本原理是什么？',
        a: '系统将每张商品图片通过 AI 模型转化为一组"特征向量"（embedding），拍照时也对照片做同样的转化，然后<strong>比对向量相似度</strong>找到最像的商品。这不是"图片比对"，而是"语义理解"——即使角度、光线不同，只要是同一件商品就能识别。',
      },
      {
        q: '如何上传识别用图片？有什么建议？',
        a: '在 <strong>全局商品库 → 编辑商品 → 识别用图片</strong> Tab 中上传。<br/><br/><strong>最佳实践：</strong><br/>• 每件商品上传 <strong>1~3 张</strong>不同角度的照片<br/>• 使用<strong>接近正方形</strong>的构图，商品居中<br/>• 背景尽量简洁，避免杂物干扰<br/>• 上传后系统自动压缩到 512×512，无需手动调整<br/>• 上传后会自动触发增量索引构建<br/><br/><strong>快速定位缺图：</strong>全局商品库列表有"识别图"列（🔴/🟡/🟢 三档），勾选"只看缺识别图的商品"可批量筛选。',
      },
      {
        q: '有几个模型可以选？该选哪个？',
        a: 'v1.1 起共 5 个可选模型，主推两个 FP16 量化版：<br/><br/>⭐ <strong>ConvNeXt V2 Pico FP16</strong>（默认推荐，约 17MB）：体积最小、下载最快，大多数场景够用<br/>⭐ <strong>DINOv2-Small FP16</strong>（高精度推荐，约 43MB）：ViT 模型，对细节最敏感，追求识别精度时选它<br/><strong>MobileCLIP-S0</strong>（约 46MB）：CLIP 家族，擅长语义理解<br/><strong>ConvNeXt / DINOv2 的 FP32 参考版</strong>：一般选 FP16 即可<br/><br/><strong>注意：</strong>模型不再打包进安装包，首次启用需联网下载，建议展会前一天完成。',
      },
      {
        q: '推理设备怎么选？CPU 和 GPU 有什么区别？',
        a: '<strong>自动模式</strong>（推荐）：系统会自动尝试 GPU 加速，不可用时降级到 CPU。<br/><strong>GPU</strong>：速度快（通常 20-50ms/张），但需要显卡支持 DirectX 12。<br/><strong>CPU</strong>：兼容性最好，速度稍慢（通常 50-200ms/张），但对识别准确率没有影响。<br/><strong>Android</strong>：自动模式走 CPU。NNAPI 可以手动选，但这几个模型在多数机型上用 NNAPI 反而更慢。<br/><br/>在 <strong>设置 → AI 视觉识别</strong> 面板可以切换推理设备。',
      },
      {
        q: '索引构建是什么？什么时候需要重建？',
        a: '<strong>索引构建</strong>就是让 AI 模型"学习"你上传的所有商品照片。<br/><br/>以下情况需要重建索引：<br/>• 上传了新的识别用图片（系统会<strong>自动增量构建</strong>）<br/>• 切换了 AI 模型（系统会提示重建）<br/>• 如果识别不准，可以手动点击<strong>"全量重建索引"</strong>刷新所有数据',
      },
      {
        q: '识别不准怎么办？',
        a: '可以尝试：<br/>1. <strong>补充图片</strong>：为识别不准的商品多上传几张不同角度的照片<br/>2. <strong>改善拍照</strong>：引导顾客将商品放在取景框中央，背景简洁<br/>3. <strong>换更大的模型</strong>：下载 MobileCLIP 或 DINOv2 试试<br/>4. <strong>全量重建</strong>：在设置点击"全量重建索引"',
      },
    ],
  },
  {
    category: '高级技巧',
    icon: SettingsOutline,
    items: [
      {
        q: '如何给顾客打折？',
        a: '两条路，覆盖现场绝大多数说法：<br/><strong>① 套装</strong>——在展会工作台的「展前 · 套装」里配好「从这几样里任选 N 件，总价 XX」，顾客的购物车会自动套用最省的那一种，并写明省了多少。<br/><strong>② 一口价</strong>——点「完成配货」后，在「确认收款」里直接改「实收」；顾客不要某个套装，也可以在这里把它拆掉。差额记成手工折让，全部算在本社团头上，帮别的社团代卖的商品仍按它们自己的定价结算。<br/>改高也可以，用于往上凑整或顺手搭了个没录入的小东西。',
      },
      {
        q: '支持“捆绑销售”或“套装”吗？',
        a: '看拆得开拆不开：<br/><strong>拆得开</strong>（5 本书装一个袋子）——在「展前 · 套装」里配一个套装。账上仍然是 5 件散货，库存精准扣减，顾客买单件也不受影响。<br/><strong>拆不开</strong>（塑封礼盒，拆了就废）——当成一个独立商品录入，进出库按它自己算。',
      },
      {
        q: '我该如何处理售罄的商品？',
        a: '在库存不足和售罄时，顾客端会自动将该商品置灰并禁止下单。您无需手动隐藏或下架商品。现场补到货了，在「展前 · 商品」点这一行的「补货」。',
      },
      {
        q: '下一场展会能沿用上一场的商品和套装吗？',
        a: '能。新展会的「展前 · 商品」里点「<strong>从上一场导入</strong>」，勾选要沿用的商品和套装；售价可以选沿用上一场或用商品库现价。整批一次导入，失败的话整批不生效。',
      },
      {
        q: '送人、弄坏了的货怎么记？',
        a: '摊主端「<strong>库存</strong>」tab 点「<strong>登记赠送/报废</strong>」。赠送默认由货主承担；算你请客的话打开「这笔我自掏」。报废只动库存不动钱。不登记的话，收摊盘点时会显示成盘点差异。',
      },
    ],
  },
  {
    category: '硬件建议',
    icon: HardwareChipOutline,
    items: [
      {
        q: '我可以在手机上运行主机端吗？',
        a: '技术上可以，但<strong>强烈不推荐</strong>。主机需要长时间运行服务，手机容易因锁屏、后台杀进程或发热导致服务中断。<strong>推荐使用笔记本电脑或平板作为主机。</strong>一定要用手机的话，请设为常亮不锁屏，并允许摊盒后台运行——锁屏或切去收款 App 时顾客下单可能卡住。',
      },
      {
        q: '对设备性能有要求吗？',
        a: '要求很低。任何能运行最新版 Chrome/Edge 的 Windows 电脑或 Android 8.0+ 的平板均可流畅运行。老旧设备作为"顾客点单机"也是完美的废物利用方案。<br/><br/>AI 拍照识别对性能要求也不高：CPU 上每张图约 50-200ms，Windows 上有 DirectX 12 显卡可进一步提速。',
      },
      {
        q: '需要一直亮屏吗？',
        a: '主机端建议保持亮屏或在电源设置中设置为“不休眠”。顾客端和平板建议在浏览器中设置“屏幕常亮”，以免顾客点单时屏幕突然熄灭。',
      },
    ],
  },
  {
    category: '社区与开源',
    icon: LogoGithub,
    items: [
      {
        q: '去哪里下载最新版本？',
        a: '请认准唯一的官方渠道：<strong>GitHub Releases</strong> 页面。我们会在那里第一时间发布新版本。任何要求付费下载或“代部署收费”的渠道均为诈骗，请勿上当。',
      },
      {
        q: '我能自动更新吗？',
        a: 'Windows 版可以：在「<strong>设置 → 关于与更新</strong>」点「检查更新」，有新版时一键下载安装并重启。<strong>正在摆摊时不要重启。</strong><br/>Android 版受系统限制，需要从发布页下载新的 APK 覆盖安装（不要先卸载，卸载会清空数据）。',
      },
      {
        q: '遇到 Bug 或有建议去哪里反馈？',
        a: '如果是程序报错，强烈建议您在 GitHub 仓库提交 <strong>Issue</strong>，这样能方便开发者追踪修复。如果是使用疑惑或单纯想闲聊，欢迎点击下方的按钮加入<strong>用户交流群</strong>。',
      },
      {
        q: '我是开发者，可以为项目贡献代码吗？',
        a: '<strong>非常欢迎！</strong> 这是一个开源项目，我们期待您提交 <strong>Pull Request</strong>。无论是修复一个小 Bug，还是开发一个全新的功能模块，您的贡献都将帮助到成千上万的摊主。',
      },
      {
        q: '我能修改软件并重新发布（甚至售卖）吗？',
        a: '本项目遵循 <strong>MIT 协议</strong>，您确实拥有修改和分发的自由。但作为一款旨在“降低同人出摊门槛”的免费工具，我们<strong>强烈不推荐</strong>将其包装为商业软件进行售卖。请保留原作者信息，尊重开源精神。',
      },
    ],
  },
]
// 搜索过滤逻辑 - 支持分类结构
const filteredQA = computed(() => {
  const query = searchQuery.value.toLowerCase().trim()
  if (!query) return qaList

  // 过滤每个分类下的问题
  return qaList
    .map((category) => ({
      ...category,
      items: category.items.filter(
        (item) => item.q.toLowerCase().includes(query) || item.a.toLowerCase().includes(query)
      ),
    }))
    .filter((category) => category.items.length > 0) // 只保留有匹配项的分类
})
// 分类展开收起逻辑
const handleCategoryChange = (names: string[]) => {
  expandedCategories.value = names
}

const toggleAllCategories = (expand: boolean) => {
  if (expand) {
    expandedCategories.value = filteredQA.value.map((c) => c.category)
  } else {
    expandedCategories.value = []
  }
}
// 统一使用共享 clipboard 工具
const copyLink = async (url: string, label: string) => {
  try {
    await copyLinkUtil(url)
    fb.success(`已复制${label}`)
  } catch (err) {
    console.error('复制失败:', err)
    fb.error(`复制${label}失败，请检查权限`)
  }
}
</script>

<style scoped>
.tutorial-container {
  max-width: var(--page-content);
  margin: 0 auto;
  padding: var(--space-xl) var(--space-lg);
  --text-primary: var(--primary-text-color);
  --text-secondary: var(--secondary-text-color);
  --bg-subtle: color-mix(in srgb, var(--text-muted) 8%, transparent);
}

/* 通用排版工具类 */
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
.mt-2 {
  margin-top: var(--space-lg);
}
.mt-4 {
  margin-top: var(--space-2xl);
}

/* Header */
.header-section {
  text-align: center;
  padding: var(--space-lg) 0;
}
.page-title {
  margin: var(--space-sm) 0 var(--space-xs);
  font-size: var(--font-2xl);
  font-weight: var(--weight-bold);
  letter-spacing: -0.5px;
  color: var(--primary-text-color);
}
.page-subtitle {
  font-size: var(--font-lg);
  color: var(--secondary-text-color);
  margin-bottom: var(--space-lg);
  line-height: 1.6;
}

/* Section General */
.section {
  margin-bottom: var(--space-2xl);
}
.section-heading {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
}
.section-heading h2 {
  margin: 0;
  font-size: var(--font-lg);
  font-weight: var(--weight-bold);
  color: var(--primary-text-color);
}

/* Carousel Tutorial */
.tutorial-carousel {
  border-radius: var(--radius-lg);
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
  padding: var(--space-xs) var(--space-md);
  border-radius: var(--radius-xl);
  font-weight: var(--weight-bold);
  font-size: var(--font-sm);
}

.step-content {
  padding: var(--space-lg);
  text-align: center;
  overflow-y: auto;
  max-height: 150px;
}

.step-content h3 {
  margin: 0 0 var(--space-sm);
  font-size: var(--font-lg);
  color: var(--primary-text-color);
}

/* 修复轮播图按钮在浅色主题下的可见性 */
:deep(.n-carousel__arrow) {
  background-color: color-mix(in srgb, var(--overlay-color) 75%, transparent) !important;
  color: var(--primary-text-color) !important;
  border-radius: var(--radius-sm);
}

:deep(.n-carousel__arrow:hover) {
  background-color: var(--overlay-color) !important;
}

:deep(.n-carousel__dots) {
  background: color-mix(in srgb, var(--overlay-color) 12%, transparent);
  padding: var(--space-sm);
  border-radius: var(--radius-md);
}

/* Quick Steps Card */
.quick-step-card {
  background: var(--bg-subtle);
  padding: var(--space-lg);
  border-radius: var(--radius-lg);
  text-align: center;
  transition: all 0.3s;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.quick-step-card:hover {
  background: color-mix(in srgb, var(--text-muted) 12%, transparent);
  transform: translateY(-2px);
}

.step-number-avatar {
  background-color: var(--bg-subtle) !important;
  color: var(--accent-color) !important;
  font-weight: var(--weight-bold);
  font-size: var(--font-xl);
  box-shadow: var(--shadow-md);
}

.quick-step-text {
  font-weight: var(--weight-bold);
  font-size: var(--font-base);
  color: var(--primary-text-color);
}

/* QA Section */
.qa-header-flex {
  justify-content: space-between;
  flex-wrap: wrap;
  gap: var(--space-lg);
}

.flex-center {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.qa-search {
  width: 240px;
}

/* QA 分类样式 */
.qa-answer {
  line-height: 1.6;
  padding: var(--space-xs) 0;
  color: var(--text-muted);
}

/* Footer */
.footer-section {
  text-align: center;
  padding-bottom: var(--space-lg);
}

.copyright {
  font-size: var(--font-sm);
  color: var(--text-muted);
  font-family: monospace;
}

/* Responsive */
@media (--phone) {
  .qa-search {
    width: 100%;
  }
  .page-title {
    font-size: var(--font-xl);
  }
  .section-heading h2 {
    font-size: var(--font-md);
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
