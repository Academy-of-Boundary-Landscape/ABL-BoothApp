<!-- docs/.vitepress/theme/components/TimelineJp.vue -->
<script setup>
const timeline = [
  {
    type: 'error',
    time: '2025年8月',
    icon: '📖',
    title: '課題：紙の記録の混乱',
    content: '夏休みに同人サークルの主催を引き継ぎ、在庫や紙の帳簿の混乱に直面。自動化ツールの必要性を痛感しました。'
  },
  {
    type: 'warning',
    time: '2025年9月〜11月',
    icon: '🧪',
    title: 'プロトタイプ：Web サービス版（Flask）',
    content: 'Flask + Vue でプロトタイプを作成しクラウドにデプロイ。ワークフローは確立できたものの、<b>ネットワーク依存が大きい</b>という壁に直面。',
    link: {
      url: 'https://github.com/Renko6626/abl_sale_tool',
      text: '初期コードのアーカイブを見る',
      title: '2025年のコミットを見る'
    }
  },
  {
    type: 'info',
    time: '2025年11月',
    icon: '💾',
    title: '試行：Nuitka パッケージ版',
    content: 'Python バックエンドの配布を試みるも、サイズ肥大と実行時依存が課題に。一般メンバー端末での運用が難しく、Windows ノートの携行も不便でした。'
  },
  {
    type: 'success',
    time: '2025年12月〜2026年5月',
    icon: '🚀',
    title: 'リファクタ：Tauri ローカルアプリ',
    content: '<b>Rust</b> でバックエンドを書き直し、<b>Tauri v2</b> で構築。Windows/Android で高性能・外部依存ゼロ・完全オフラインを実現。'
  },
  {
    type: 'success',
    time: '2026年9月',
    icon: '📒',
    title: 'v1.2：複式簿記の帳簿',
    content: '帳簿を<b>複式簿記</b>で作り直し、モノとお金の出入りをすべて記録。セット割引、撤収時の精算と委託販売の分配、バーコードスキャンを追加し、管理画面と売り子画面をイベントの流れ（事前準備／当日／撤収）に沿って再設計しました。'
  }
]
</script>

<template>
  <div class="bk-timeline-wrapper">
    <div class="bk-timeline-header">
      <div class="bk-icon-box">
        <span class="bk-header-icon">📖</span>
      </div>
      <div class="bk-header-text">
        <h2 class="bk-title">開発の歩み</h2>
        <p class="bk-subtitle">プロトタイプから完成された出店アプリへ</p>
      </div>
    </div>

    <div class="bk-timeline-body">
      <div class="bk-timeline">
        <div 
          v-for="(item, index) in timeline" 
          :key="index"
          class="bk-timeline-item"
          :class="`bk-type-${item.type}`"
        >
          <div class="bk-timeline-line"></div>
          <div class="bk-timeline-icon">
            <span class="bk-icon">{{ item.icon }}</span>
          </div>
          <div class="bk-timeline-content">
            <div class="bk-timeline-time">{{ item.time }}</div>
            <div class="bk-timeline-header">
              <span class="bk-tl-title">{{ item.title }}</span>
              <a 
                v-if="item.link"
                :href="item.link.url" 
                target="_blank" 
                class="bk-archive-link"
                :title="item.link.title"
              >
                <span class="bk-link-icon">🏛️</span> {{ item.link.text }}
              </a>
            </div>
            <p class="bk-tl-content" v-html="item.content"></p>
          </div>
        </div>
      </div>
    </div>
    
    <div class="bk-timeline-footer">
      <small>🚩 オリジナルから始まり、進化し続ける</small>
    </div>
  </div>
</template>

<style scoped>
.bk-timeline-wrapper {
  margin: 4rem auto;
  max-width: 800px;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  padding: 2rem;
  box-shadow: 0 4px 24px rgba(0,0,0,0.04);
}

.bk-timeline-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 2rem;
  padding-bottom: 1rem;
  border-bottom: 1px dashed var(--vp-c-divider);
}

.bk-icon-box {
  background: var(--vp-c-bg);
  padding: 8px;
  border-radius: 10px;
  border: 1px solid var(--vp-c-divider);
  display: flex;
}

.bk-header-icon {
  font-size: 24px;
  line-height: 1;
}

.bk-title {
  margin: 0 !important;
  font-size: 1.4rem;
  font-weight: 700;
  border: none !important;
  line-height: 1.2;
}

.bk-subtitle {
  margin: 0.2rem 0 0 0;
  font-size: 0.9rem;
  color: var(--vp-c-text-2);
}

.bk-timeline-body { padding: 0 0.5rem; }
.bk-timeline { position: relative; }

.bk-timeline-item {
  position: relative;
  padding-left: 3rem;
  padding-bottom: 2rem;
}
.bk-timeline-item:last-child { padding-bottom: 0; }
.bk-timeline-item:last-child .bk-timeline-line { display: none; }

.bk-timeline-line {
  position: absolute;
  left: 1rem;
  top: 2rem;
  bottom: 0;
  width: 2px;
  background: var(--vp-c-divider);
}

.bk-timeline-icon {
  position: absolute;
  left: 0;
  top: 0;
  width: 2rem;
  height: 2rem;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  z-index: 1;
}

.bk-type-error .bk-timeline-icon { border-color: #d03050; background: rgba(208, 48, 80, 0.1); }
.bk-type-warning .bk-timeline-icon { border-color: #f0a020; background: rgba(240, 160, 32, 0.1); }
.bk-type-info .bk-timeline-icon { border-color: #3451b2; background: rgba(52, 81, 178, 0.1); }
.bk-type-success .bk-timeline-icon { border-color: #18a058; background: rgba(24, 160, 88, 0.1); }

.bk-icon { font-size: 1rem; line-height: 1; }
.bk-timeline-content { padding-top: 0.1rem; }

.bk-timeline-time { font-size: 0.85rem; color: var(--vp-c-text-3); margin-bottom: 0.3rem; font-weight: 500; }

.bk-timeline-header { display: flex; align-items: center; flex-wrap: wrap; gap: 0.8rem; margin-bottom: 0.5rem; }
.bk-tl-title { font-weight: 700; font-size: 1.05rem; color: var(--vp-c-text-1); }
.bk-tl-content { margin: 0.25rem 0 0 0; font-size: 0.95rem; line-height: 1.6; color: var(--vp-c-text-2); }

.bk-timeline-footer { margin-top: 1.5rem; text-align: right; color: var(--vp-c-text-3); font-weight: 500; opacity: 0.7; }

.bk-archive-link { display: inline-flex; align-items: center; gap: 4px; font-size: 0.8rem; text-decoration: none; background: var(--vp-c-bg-alt); border: 1px solid var(--vp-c-divider); padding: 2px 8px; border-radius: 4px; color: var(--vp-c-text-2); transition: all 0.2s ease; }
.bk-archive-link:hover { border-color: var(--vp-c-brand); color: var(--vp-c-brand); background: var(--vp-c-bg); }

/* Mobile */
@media (max-width: 640px) {
  .bk-timeline-wrapper { padding: 1.5rem 1rem; margin: 2rem 0; border-radius: 12px; }
  .bk-timeline-item { padding-left: 2.5rem; }
  .bk-timeline-line { left: 0.75rem; }
  .bk-timeline-icon { width: 1.5rem; height: 1.5rem; }
  .bk-icon { font-size: 0.85rem; }
}
</style>
