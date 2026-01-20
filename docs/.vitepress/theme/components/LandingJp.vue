<!-- docs/.vitepress/theme/components/LandingJp.vue -->
<template>
  <div class="bk-landing">
    <section class="bk-kpis">
      <div class="bk-kpi">
        <div class="bk-kpi-title">適用シーン</div>
        <div class="bk-kpi-value">同人イベント出店 / オフラインイベント</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">ネットワーク要件</div>
        <div class="bk-kpi-value">LAN のみ（テザリングでOK）</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">データ保存</div>
        <div class="bk-kpi-value">ローカル（Excel にエクスポート可）</div>
      </div>
      <div class="bk-kpi">
        <div class="bk-kpi-title">ライセンス</div>
        <div class="bk-kpi-value">MIT ・ 永久無料</div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🚫 なぜ Booth‑Kernel を開発したのか？</h2>
        <p class="bk-sub">サークル運営の中で様々な方法を試し、最終的にこの課題に直面しました。</p>
      </header>

      <div class="bk-grid-3">
        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">📝</span>
            <h3 class="bk-h3">紙での手書き管理</h3>
          </div>
          <ul class="bk-list">
            <li><b>計算ミスが発生しやすい：</b>複数商品のセット価格は忙しい場面で口頭計算が難しく、電卓でも時間を取られます。</li>
            <li><b>在庫管理ミス：</b>会計・受け渡し・記帳・在庫線引きを同時に行うと、どうしてもミスが増えます。</li>
            <li><b>振り返りが困難：</b>イベント後、メモは読みづらく、対帳や分析に時間がかかります。</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">🌐</span>
            <h3 class="bk-h3">WeChat / Alipay ミニプログラム</h3>
          </div>
          <ul class="bk-list">
            <li><b>ネット依存：</b>会場のネットワークは不安定で、重いアセットの読み込みが遅くなりがち。</li>
            <li><b>初期設定が煩雑：</b>認証や設定のハードルが高く、イベントごとに手続きが必要。</li>
            <li><b>エコシステム分断：</b>WeChat と Alipay は支払い・データが相互運用できず、体験が分断されます。</li>
            <li><b>認知されづらい：</b>ミニプログラム QR は目立ちにくく、お客様が見つけづらい。</li>
          </ul>
        </article>

        <article class="bk-card bk-card-bad">
          <div class="bk-card-top">
            <span class="bk-emoji">☁️</span>
            <h3 class="bk-h3">Web 出店機（初期案）</h3>
          </div>
          <ul class="bk-list">
            <li><b>データリスク：</b>中央集権サーバーは複数サークル環境でセンシティブ。商品・売上データは外部と共有すべきではありません。</li>
            <li><b>導入が複雑：</b>サーバー・ドメイン・HTTPS 構築は非エンジニアに不親切でコストも高い。</li>
            <li><b>オフラインに弱い：</b>ネットワークやサーバー障害でブース運営が止まる恐れがあります。</li>
          </ul>
        </article>
      </div>
    </section>

    <section class="bk-section bk-solution">
      <header class="bk-section-header">
        <h2 class="bk-h2">✅ 私たちの答え：ローカル App + LAN 接続</h2>
        <p class="bk-sub">
          Booth‑Kernel は <b>ローカルデータ</b> と <b>LAN 同期</b> が中核：テザリングで即時ネットワーク、複数端末で協調、オフラインでも堅牢です。
        </p>
      </header>

      <div class="bk-grid-2">
        <div class="bk-highlight">
          <h3 class="bk-h3">📶 オフライン LAN アーキテクチャ</h3>
          <p class="bk-p">インターネット不要。スマホのテザリング → 端末接続 → 即同期。現場で安定。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">⚡ 出店者に寄り添うワークフロー</h3>
          <p class="bk-p">受付 → 確認 → ピッキング。<b>在庫控除</b>・<b>売上記録</b>・<b>レポート生成</b>を自動化。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🎨 お客様が注文しやすい</h3>
          <p class="bk-p">タブレットを「電子メニュー」に：大きな画像、明瞭な価格、素早い注文で行列を緩和。</p>
        </div>
        <div class="bk-highlight">
          <h3 class="bk-h3">🔒 データはあなたの手元に</h3>
          <p class="bk-p">クラウド依存なし。データは端末内に留まり、Excel への一括エクスポートも可能。</p>
        </div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🔄 現場ではどう動かす？</h2>
        <p class="bk-sub">最小 2 台で運用可能。出店者・タブレットの増設にも柔軟に拡張できます。</p>
      </header>

      <div class="bk-workflow">
        <div class="bk-node">
          <div class="bk-node-head bk-host">管理端（PC / タブレット / スマホ）</div>
          <div class="bk-node-body">
            <div class="bk-node-title">グローバルダッシュボード</div>
            <div class="bk-node-desc">商品登録 ・ 在庫設定 ・ 注文管理 ・ レポート出力</div>
          </div>
        </div>

        <div class="bk-link">
          <span class="bk-link-pill">Wi‑Fi LAN 同期</span>
        </div>

        <div class="bk-branch">
          <div class="bk-node">
            <div class="bk-node-head bk-customer">顧客端（タブレット）</div>
            <div class="bk-node-body">
              <div class="bk-node-title">セルフ注文端末</div>
              <div class="bk-node-desc">大きな画像 ・ カート追加 ・ QR 決済</div>
            </div>
          </div>

          <div class="bk-node">
            <div class="bk-node-head bk-vendor">出店者端（スマホ）</div>
            <div class="bk-node-body">
              <div class="bk-node-title">ピッキング通知</div>
              <div class="bk-node-desc">注文通知 ・ 入金確認 ・ ピッキング支援</div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="bk-section">
      <header class="bk-section-header">
        <h2 class="bk-h2">🎨 UI プレビュー</h2>
        <p class="bk-sub">横向き 2 枚（顧客 / 統計）、縦向き 1 枚（出店者）。</p>
      </header>

      <div class="bk-shots3">
        <!-- 横向き 1：統計 -->
        <figure class="bk-shot bk-shot--landscape">
          <img src="/images/stat.png" alt="統計ダッシュボードのスクリーンショット" loading="lazy" />
          <figcaption>統計：売上・人気商品・在庫推移が一目で分かる</figcaption>
        </figure>

        <!-- 横向き 2：顧客 -->
        <figure class="bk-shot bk-shot--landscape">
          <img src="/images/customer.png" alt="顧客向け注文画面のスクリーンショット" loading="lazy" />
          <figcaption>顧客：大きな画像 + カート追加 + QR 決済</figcaption>
        </figure>

        <!-- 縦向き：出店者 -->
        <figure class="bk-shot bk-shot--portrait">
          <img src="/images/vendor.png" alt="出店者向けピッキング画面のスクリーンショット" loading="lazy" />
          <figcaption>出店者：注文通知、入金確認、ピッキング支援</figcaption>
        </figure>
      </div>
    </section>

    <section class="bk-cta">
      <h2 class="bk-h2">出店をもっと楽しく</h2>
      <p class="bk-sub">面倒はシステムに任せて、熱意はファンへ。</p>

      <div class="bk-buttons">
        <a class="bk-btn bk-btn-primary" href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases">
          💾 Windows / Android をダウンロード
        </a>
        <a class="bk-btn bk-btn-ghost" href="/ja/guide/getting-started">
          📖 クイックスタートを読む
        </a>
      </div>

      <p class="bk-foot">
        <a href="https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp" target="_blank" rel="noreferrer">
          100% オープンソース ・ MIT ・ 永久無料
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
  .bk-shots3{ grid-template-columns: 1fr; }
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
.bk-shot--landscape img{ aspect-ratio: 16 / 9; object-fit: contain; background: var(--vp-c-bg); }
.bk-shot--portrait img{ aspect-ratio: 9 / 16; object-fit: contain; background: var(--vp-c-bg); }

/* CTA */
.bk-cta{
  text-align:center;
  margin: 4.5rem 0 3rem;
  padding: 2.4rem 1.2rem;
  border-radius: 20px;
  background: linear-gradient(180deg, color-mix(in oklab, var(--vp-c-brand) 14%, transparent), transparent);
  border: 1px solid var(--vp-c-divider);
}
.bk-buttons{ display:flex; gap: .9rem; justify-content:center; flex-wrap: wrap; margin-top: 1.4rem; }
.bk-btn{
  display:inline-flex; align-items:center; justify-content:center; gap: .5rem;
  padding: .72rem 1.15rem; border-radius: 999px; text-decoration: none !important;
  font-weight: 800; border: 1px solid var(--vp-c-divider);
  transition: transform .15s ease, background .15s ease, border-color .15s ease;
}
.bk-btn-primary{ background: var(--vp-c-brand); color: #fff !important; border-color: color-mix(in oklab, var(--vp-c-brand) 75%, var(--vp-c-divider)); box-shadow: 0 12px 34px rgba(0,0,0,0.16); }
.bk-btn-ghost{ background: var(--vp-c-bg); color: var(--vp-c-brand) !important; border-color: color-mix(in oklab, var(--vp-c-brand) 35%, var(--vp-c-divider)); }
.bk-btn:hover{ transform: translateY(-2px); }
.bk-foot{ margin-top: 1.25rem; font-size: .92rem; color: var(--vp-c-text-2); }
.bk-foot a{ color: var(--vp-c-text-2) !important; text-decoration: none; }
.bk-foot a:hover{ color: var(--vp-c-brand) !important; }
</style>
