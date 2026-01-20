<!-- docs/.vitepress/theme/components/Landing.vue -->
<template>
  <div class="bk-landing">
    <section class="bk-kpis">
      <div class="bk-kpi">
        <div class="bk-kpi-title">适用场景</div>
        <div class="bk-kpi-value">漫展出摊 / 线下活动</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">网络要求</div>
        <div class="bk-kpi-value">仅局域网（热点即可）</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">数据存储</div>
        <div class="bk-kpi-value">本地（可导出 Excel）</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">开源协议</div>
        <div class="bk-kpi-value">MIT · 永久免费</div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🚫 为什么我们要开发摊盒APP？</h2>
        <p class="bk-sub">我们在社团经营过程中试过很多方案，最终踩过这些坑。</p>
      </header>

      <div class="bk-grid-3">
        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">📝</span>
            <h3 class="bk-h3">传统纸笔记账</h3>
          </div>
          <ul class="bk-list">
            <li><b>算数易错：</b>多件商品组合计算价格，忙起来口算很难保证准确，而敲计算器同样费时费力。</li>
            <li><b>库存易错：</b>摊主同时算钱-递货-记账-记库存，手忙脚乱地在纸上画正字极易出错</li>
            <li><b>复盘困难：</b>回去面对一堆草稿纸上面的抽象笔记和潦草记录，难对账、难分析。</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">🌐</span>
            <h3 class="bk-h3">微信 / 支付宝 点菜小程序</h3>
          </div>
          <ul class="bk-list">
            <li><b>依赖网络：</b>漫展网络复杂，小程序需要的大量组件和图像素材可能无法快速加载。</li>
            <li><b>前期配置繁琐：</b>认证、资料、配置门槛高，每场展会都需要现场填写信息。</li>
            <li><b>生态割裂：</b>微信支付宝的点单小程序支付方式和数据完全不互通，体验割裂和麻烦。</li>
            <li><b>展示困难：</b>只能贴一个“小程序码”，它非常不显眼，顾客容易找不到。</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">☁️</span>
            <h3 class="bk-h3">Web 出摊机（早期方案）</h3>
          </div>
          <ul class="bk-list">
            <li><b>数据风险：</b>中心化服务器在多社团场景更敏感，制品信息和销售数据不能和他人共享。</li>
            <li><b>部署复杂：</b>即使有了自动化部署脚本，配置云服务器、域名、HTTPS 对非技术爱好者非常不友好且价格昂贵。</li>
            <li><b>断网即死：</b>在复杂的网络环境之下，同样面临服务器或链路出问题，摊位直接瘫痪的问题。</li>
          </ul>
        </article>
      </div>
    </section>

    <section class="bk-section bk-solution">
      <header class="bk-section-header">
        <h2 class="bk-h2">✅ 我们的答案：本地 App + 局域网连接</h2>
        <p class="bk-sub">
          摊盒App 以 <b>本地数据</b> + <b>局域网同步</b> 为核心：开热点就能组网，多设备协同，离线也稳定。
        </p>
      </header>

      <div class="bk-grid-2">
        <div class="bk-highlight">
          <h3 class="bk-h3">📶 离线局域网架构</h3>
          <p class="bk-p">无需互联网。手机开热点（不耗流量）→ 设备连入 → 立即同步，现场更稳。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">⚡ 摊主友好的工作流</h3>
          <p class="bk-p">接单 → 确认 → 配货；系统自动做 <b>扣库存</b>、<b>记流水</b>、<b>生成报表</b>。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🎨 顾客更愿意下单</h3>
          <p class="bk-p">平板做“电子菜单”：大图、清晰价格、快速下单，减少沟通成本与拥堵。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🔒 数据安全可控</h3>
          <p class="bk-p">没有中心化云端。数据只在你的设备里，并支持一键导出 Excel 做复盘。</p>
        </div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🔄 出摊现场怎么跑起来？</h2>
        <p class="bk-sub">最少两台设备即可完整工作；多摊主、多平板也能扩展协同。</p>
      </header>

      <div class="bk-workflow">
        <div class="bk-node">
          <div class="bk-node-head bk-host">管理端（电脑 / 平板 / 手机）</div>
          <div class="bk-node-body">
            <div class="bk-node-title">全局控制台</div>
            <div class="bk-node-desc">商品录入 · 库存设置 · 订单管理 · 报表导出</div>
          </div>
        </div>

        <div class="bk-link">
          <span class="bk-link-pill">Wi-Fi 局域网同步</span>
        </div>

        <div class="bk-branch">
          <div class="bk-node">
            <div class="bk-node-head bk-customer">顾客端（平板）</div>
            <div class="bk-node-body">
              <div class="bk-node-title">自助点单机</div>
              <div class="bk-node-desc">大图浏览 · 自助加购 · 扫码支付</div>
            </div>
          </div>

          <div class="bk-node">
            <div class="bk-node-head bk-vendor">摊主端（手机）</div>
            <div class="bk-node-body">
              <div class="bk-node-title">配货提醒器</div>
              <div class="bk-node-desc">接单提醒 · 确认收款 · 配货提示</div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="bk-section">
    <header class="bk-section-header">
        <h2 class="bk-h2">🎨 界面预览</h2>
        <p class="bk-sub">两张横屏展示“顾客点单/统计报表”，一张竖屏展示“摊主端操作”。</p>
    </header>

    <div class="bk-shots3">
        <!-- 横屏 1：统计 -->
        <figure class="bk-shot bk-shot--landscape">
        <img src="/images/stat.png" alt="统计报表页面截图" loading="lazy" />
        <figcaption>统计报表：流水、热销、库存变化一目了然</figcaption>
        </figure>

        <!-- 横屏 2：顾客 -->
        <figure class="bk-shot bk-shot--landscape">
        <img src="/images/customer.png" alt="顾客自助点单页面截图" loading="lazy" />
        <figcaption>顾客端：大图浏览 + 自助加购 + 扫码支付</figcaption>
        </figure>

        <!-- 竖屏：摊主 -->
        <figure class="bk-shot bk-shot--portrait">
        <img src="/images/vendor.png" alt="摊主配货页面截图" loading="lazy" />
        <figcaption>摊主端：接单提醒、确认收款、配货提示</figcaption>
        </figure>
    </div>
    </section>
    <section class="bk-cta">
      <h2 class="bk-h2">让出摊回归快乐本身</h2>
      <p class="bk-sub">把繁琐交给系统，把热情留给同好。</p>

      <div class="bk-buttons">
        <a class="bk-btn bk-btn-primary" href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases">
          💾 下载 Windows / Android
        </a>
        <a class="bk-btn bk-btn-ghost" href="/guide/getting-started">
          📖 阅读使用教程
        </a>
      </div>

      <p class="bk-foot">
        <a href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp" target="_blank" rel="noreferrer">
          100% 开源 · MIT 协议 · 永久免费
        </a>
      </p>
    </section>
  </div>
</template>

<style scoped>
.bk-landing{ padding: 2rem 1rem; max-width: 1120px; margin: 0 auto; }

/* Typography */
.bk-h2{
  text-align: center;
  border: none !important;
  margin: 0 0 0.75rem !important;
  font-weight: 800;
  font-size: clamp(1.6rem, 2.2vw, 2.2rem);
  line-height: 1.25;
}
.bk-h3{ margin: 0; font-size: 1.05rem; font-weight: 750; }
.bk-sub{
  text-align: center;
  color: var(--vp-c-text-2);
  margin: 0 auto;
  max-width: 760px;
  font-size: 1.05rem;
  line-height: 1.7;
}
.bk-p{ margin: 0; line-height: 1.7; color: var(--vp-c-text-2); }

/* Section spacing */
.bk-section{ margin-top: 3.75rem; }
.bk-section-header{ margin-bottom: 1.75rem; }

/* KPI row */
.bk-kpis{
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.9rem;
  margin-top: 1.25rem;
}
@media (max-width: 900px){
  .bk-kpis{ grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
.bk-kpi{
  padding: 1rem 1rem;
  border-radius: 14px;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
}
.bk-kpi-title{
  font-size: 0.85rem;
  color: var(--vp-c-text-3);
  margin-bottom: 0.25rem;
}
.bk-kpi-value{
  font-weight: 700;
  color: var(--vp-c-text-1);
  font-size: 0.95rem;
}

/* Cards grid */
.bk-grid-3{
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1.1rem;
}
@media (max-width: 960px){
  .bk-grid-3{ grid-template-columns: 1fr; }
}
.bk-card{
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  padding: 1.4rem 1.35rem;
  transition: transform .15s ease, box-shadow .15s ease;
}
.bk-card:hover{
  transform: translateY(-3px);
  box-shadow: 0 10px 30px rgba(0,0,0,0.06);
}
.bk-card-top{
  display:flex;
  align-items:center;
  gap: .65rem;
  margin-bottom: .95rem;
}
.bk-emoji{ font-size: 1.7rem; }

/* Bad card accent */
.bk-card-bad{ position: relative; }
.bk-card-bad::before{
  content:"";
  position:absolute;
  inset: 0;
  border-radius: 16px;
  pointer-events:none;
  background: linear-gradient(135deg, rgba(255, 99, 99, .12), rgba(255, 99, 99, 0));
}

/* Lists */
.bk-list{
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: .65rem;
}
.bk-list li{
  color: var(--vp-c-text-2);
  line-height: 1.65;
  font-size: .95rem;
}
.bk-list b{ color: var(--vp-c-text-1); font-weight: 750; }

/* Highlights */
.bk-solution{
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 20px;
  padding: 2.2rem 1.2rem;
}
.bk-grid-2{
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
  margin-top: 1.5rem;
}
@media (max-width: 960px){
  .bk-grid-2{ grid-template-columns: 1fr; }
}
.bk-highlight{
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-divider);
  border-radius: 14px;
  padding: 1.2rem 1.1rem;
}
.bk-highlight .bk-h3{ margin-bottom: .4rem; color: var(--vp-c-brand); }

/* Workflow */
.bk-workflow{
  display:flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
}
.bk-node{
  width: min(860px, 100%);
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  overflow: hidden;
}
.bk-node-head{
  padding: .8rem 1rem;
  font-weight: 800;
  color: var(--vp-c-bg);
}
.bk-host{ background: color-mix(in oklab, var(--vp-c-brand) 88%, #000 12%); }
.bk-customer{ background: color-mix(in oklab, #10b981 85%, #000 15%); }
.bk-vendor{ background: color-mix(in oklab, #f59e0b 85%, #000 15%); }
.bk-node-body{ padding: 1.05rem 1rem 1.15rem; }
.bk-node-title{ font-weight: 850; font-size: 1.05rem; margin-bottom: .25rem; }
.bk-node-desc{ color: var(--vp-c-text-2); line-height: 1.7; }

.bk-link{ width: min(860px, 100%); display:flex; justify-content:center; }
.bk-link-pill{
  border: 1px dashed var(--vp-c-divider);
  background: var(--vp-c-bg);
  color: var(--vp-c-text-2);
  padding: .55rem .85rem;
  border-radius: 999px;
  font-weight: 700;
  font-size: .9rem;
}
.bk-branch{
  width: min(860px, 100%);
  display:grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}
@media (max-width: 960px){
  .bk-branch{ grid-template-columns: 1fr; }
}

/* Screenshots */
.bk-shots3{
  display: grid;
  gap: 1.2rem;
  align-items: start;
  margin-top: 1.25rem;
}

/* 桌面：两横并排 + 竖屏居中一行（更像产品页） */
@media (min-width: 960px){
  .bk-shots3{
    grid-template-columns: 1fr 1fr;
    grid-template-areas:
      "land1 land2"
      "port  port";
  }
  .bk-shots3 > figure:nth-child(1){ grid-area: land1; }
  .bk-shots3 > figure:nth-child(2){ grid-area: land2; }
  .bk-shots3 > figure:nth-child(3){
    grid-area: port;
    justify-self: center;
    width: min(420px, 100%); /* 竖屏不要铺满 */
  }
}

@media (max-width: 959px){
  .bk-shots3{
    grid-template-columns: 1fr;
  }
}

/* Card */
.bk-shot{
  margin: 0;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  padding: 1rem;
}

/* Image base */
.bk-shot img{
  width: 100%;
  height: auto;
  display:block;
  border-radius: 12px;
  border: 1px solid var(--vp-c-divider);
  box-shadow: 0 14px 45px rgba(0,0,0,0.14);
  transition: transform .2s ease;
}
.bk-shot img:hover{ transform: scale(1.01); }

.bk-shot figcaption{
  margin-top: .75rem;
  color: var(--vp-c-text-2);
  font-size: .92rem;
  line-height: 1.6;
}
.bk-shot--landscape img{
  aspect-ratio: 16 / 9;
  object-fit: contain; /* 保证不裁掉内容 */
  background: var(--vp-c-bg);
}

.bk-shot--portrait img{
  aspect-ratio: 9 / 16;
  object-fit: contain;
  background: var(--vp-c-bg);
}
/* CTA */
.bk-cta{
  text-align:center;
  margin: 4.5rem 0 3rem;
  padding: 2.4rem 1.2rem;
  border-radius: 20px;
  background: linear-gradient(180deg, color-mix(in oklab, var(--vp-c-brand) 14%, transparent), transparent);
  border: 1px solid var(--vp-c-divider);
}
.bk-buttons{
  display:flex;
  gap: .9rem;
  justify-content:center;
  flex-wrap: wrap;
  margin-top: 1.4rem;
}
.bk-btn{
  display:inline-flex;
  align-items:center;
  justify-content:center;
  gap: .5rem;
  padding: .72rem 1.15rem;
  border-radius: 999px;
  text-decoration: none !important;
  font-weight: 800;
  border: 1px solid var(--vp-c-divider);
  transition: transform .15s ease, background .15s ease, border-color .15s ease;
}
.bk-btn-primary{
  background: var(--vp-c-brand);
  color: white !important;
  border-color: color-mix(in oklab, var(--vp-c-brand) 75%, var(--vp-c-divider));
  box-shadow: 0 12px 34px rgba(0,0,0,0.16);
}
.bk-btn-ghost{
  background: var(--vp-c-bg);
  color: var(--vp-c-brand) !important;
  border-color: color-mix(in oklab, var(--vp-c-brand) 35%, var(--vp-c-divider));
}
.bk-btn:hover{ transform: translateY(-2px); }
.bk-foot{
  margin-top: 1.25rem;
  font-size: .92rem;
  color: var(--vp-c-text-2);
}
.bk-foot a{
  color: var(--vp-c-text-2) !important;
  text-decoration: none;
}
.bk-foot a:hover{ color: var(--vp-c-brand) !important; }
</style>
