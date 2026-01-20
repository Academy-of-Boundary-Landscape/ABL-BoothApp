<!-- docs/.vitepress/theme/components/LandingEn.vue -->
<template>
  <div class="bk-landing">
    <section class="bk-kpis">
      <div class="bk-kpi">
        <div class="bk-kpi-title">Use Cases</div>
        <div class="bk-kpi-value">Comic conventions / offline events</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">Network</div>
        <div class="bk-kpi-value">LAN only (phone hotspot works)</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">Data Storage</div>
        <div class="bk-kpi-value">Local (exportable to Excel)</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">License</div>
        <div class="bk-kpi-value">MIT · Free forever</div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🚫 Why did we build Booth-Kernel?</h2>
        <p class="bk-sub">We tried many approaches running a fan club booth and hit these pitfalls.</p>
      </header>

      <div class="bk-grid-3">
        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">📝</span>
            <h3 class="bk-h3">Paper-based bookkeeping</h3>
          </div>
          <ul class="bk-list">
            <li><b>Error-prone math:</b> Bundle pricing is hard to calculate under pressure; even calculators slow you down.</li>
            <li><b>Inventory mistakes:</b> Juggling cash, handover, notes, and stock marking leads to errors quickly.</li>
            <li><b>Hard to review:</b> Messy notes make post-event reconciliation and analysis painful.</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">🌐</span>
            <h3 class="bk-h3">WeChat/Alipay mini-program ordering</h3>
          </div>
          <ul class="bk-list">
            <li><b>Network dependent:</b> Convention networks are complex; heavy assets often fail to load fast enough.</li>
            <li><b>Complicated setup:</b> Certification and configuration are high-friction; every event needs fresh forms.</li>
            <li><b>Split ecosystems:</b> WeChat and Alipay payment/data are not interoperable — fragmented experience.</li>
            <li><b>Poor visibility:</b> Mini-program QR codes are easy to miss; customers can’t find them.</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">☁️</span>
            <h3 class="bk-h3">Web-based booth system (early attempt)</h3>
          </div>
          <ul class="bk-list">
            <li><b>Data risk:</b> Centralized servers are sensitive in multi-club contexts; product and sales data must stay private.</li>
            <li><b>Complex deployment:</b> Cloud servers, domains, HTTPS — unfriendly and costly for non-tech users.</li>
            <li><b>Offline failure:</b> Server or network issues can halt your booth entirely.</li>
          </ul>
        </article>
      </div>
    </section>

    <section class="bk-section bk-solution">
      <header class="bk-section-header">
        <h2 class="bk-h2">✅ Our answer: Local app + LAN sync</h2>
        <p class="bk-sub">
          Booth-Kernel centers on <b>local data</b> + <b>LAN synchronization</b>: enable phone hotspot, connect devices, and collaborate — rock solid even offline.
        </p>
      </header>

      <div class="bk-grid-2">
        <div class="bk-highlight">
          <h3 class="bk-h3">📶 Offline LAN architecture</h3>
          <p class="bk-p">No internet needed. Turn on phone hotspot (no data usage) → connect devices → sync instantly.</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">⚡ Workflow built for vendors</h3>
          <p class="bk-p">Order → Confirm → Fulfill; the system automatically <b>deducts stock</b>, <b>logs sales</b>, and <b>generates reports</b>.</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🎨 Easier for customers to order</h3>
          <p class="bk-p">Tablet as "digital menu": big images, clear pricing, quick ordering — less friction and crowding.</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🔒 Data stays under your control</h3>
          <p class="bk-p">No centralized cloud. Data stays on your devices, with one-click Excel export for review.</p>
        </div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🔄 How does it run on-site?</h2>
        <p class="bk-sub">Two devices are enough for a full workflow; multiple tablets/phones scale easily.</p>
      </header>

      <div class="bk-workflow">
        <div class="bk-node">
          <div class="bk-node-head bk-host">Admin (PC / tablet / phone)</div>
          <div class="bk-node-body">
            <div class="bk-node-title">Global dashboard</div>
            <div class="bk-node-desc">Product entry · Stock settings · Order management · Report export</div>
          </div>
        </div>

        <div class="bk-link">
          <span class="bk-link-pill">Wi‑Fi LAN sync</span>
        </div>

        <div class="bk-branch">
          <div class="bk-node">
            <div class="bk-node-head bk-customer">Customer (tablet)</div>
            <div class="bk-node-body">
              <div class="bk-node-title">Self-service ordering</div>
              <div class="bk-node-desc">Large images · add-to-cart · QR payments</div>
            </div>
          </div>

          <div class="bk-node">
            <div class="bk-node-head bk-vendor">Vendor (phone)</div>
            <div class="bk-node-body">
              <div class="bk-node-title">Fulfillment notifier</div>
              <div class="bk-node-desc">Order alerts · payment confirmation · picking guidance</div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🎨 UI Preview</h2>
        <p class="bk-sub">Two landscape shots (customer/statistics) and one portrait (vendor).</p>
      </header>

      <div class="bk-shots3">
        <!-- Landscape 1: Stats -->
        <figure class="bk-shot bk-shot--landscape">
          <img src="/images/stat.png" alt="Statistics dashboard screenshot" loading="lazy" />
          <figcaption>Statistics: revenue, best sellers, and stock changes at a glance</figcaption>
        </figure>

        <!-- Landscape 2: Customer -->
        <figure class="bk-shot bk-shot--landscape">
          <img src="/images/customer.png" alt="Customer ordering screen screenshot" loading="lazy" />
          <figcaption>Customer: large images + self-service cart + QR payment</figcaption>
        </figure>

        <!-- Portrait: Vendor -->
        <figure class="bk-shot bk-shot--portrait">
          <img src="/images/vendor.png" alt="Vendor picking screen screenshot" loading="lazy" />
          <figcaption>Vendor: order alerts, payment confirmation, and picking prompts</figcaption>
        </figure>
      </div>
    </section>

    <section class="bk-cta">
      <h2 class="bk-h2">Make selling fun again</h2>
      <p class="bk-sub">Let the system handle the tedious parts — keep the passion for fans.</p>

      <div class="bk-buttons">
        <a class="bk-btn bk-btn-primary" href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases">
          💾 Download for Windows / Android
        </a>
        <a class="bk-btn bk-btn-ghost" href="/en/guide/getting-started">
          📖 Read the quickstart
        </a>
      </div>

      <p class="bk-foot">
        <a href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp" target="_blank" rel="noreferrer">
          100% open source · MIT · Free forever
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

/* Desktop: two landscape + one portrait centered */
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
    width: min(420px, 100%);
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
  object-fit: contain;
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
