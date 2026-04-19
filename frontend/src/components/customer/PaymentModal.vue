<template>
  <Transition name="payment-fade">
    <div v-if="show" class="payment-overlay">
      <div class="payment-card">
        <!-- 顶部：金额 -->
        <div class="payment-header">
          请扫码支付 <strong>¥{{ total.toFixed(2) }}</strong>
        </div>

        <!-- 中间：二维码区域 -->
        <div class="qr-area">
          <!-- 多码并排 -->
          <div v-if="qrCodeUrls.length > 0" class="qr-grid" :class="{ 'single': qrCodeUrls.length === 1 }">
            <div v-for="(url, i) in qrCodeUrls" :key="i" class="qr-wrapper">
              <img :src="url" :alt="'收款码 ' + (i + 1)" class="qr-img" />
            </div>
          </div>

          <div v-else class="no-qr">
            <p>暂无收款码</p>
            <p>请联系摊主设置收款码</p>
          </div>
        </div>

        <!-- 底部：提示 + 关闭按钮 -->
        <div class="payment-footer">
          <p class="scan-tip">
            手机浏览器用户请长按二维码保存后，用微信/支付宝扫一扫
          </p>
          <div class="timer-bar">
            <div class="timer-fill" :style="{ width: progress + '%' }"></div>
          </div>
          <n-button
            type="primary"
            block
            round
            size="large"
            class="close-btn"
            @click="handleClose"
          >
            确认已付款 · 关闭{{ countdown > 0 && countdown <= 30 ? `（${countdown}s）` : '' }}
          </n-button>
          <button v-if="countdown > 0 && countdown <= 30" class="extend-btn" @click="resetCountdown">
            需要更多时间
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, computed, watch, onUnmounted } from 'vue'
import { NButton } from 'naive-ui'

const AUTO_CLOSE_SECONDS = 90

const emit = defineEmits(['close'])
const props = defineProps({
  show: { type: Boolean, required: true },
  total: { type: Number, required: true },
  qrCodeUrls: { type: Array, default: () => [] }
})

const countdown = ref(0)
const progress = computed(() => (countdown.value / AUTO_CLOSE_SECONDS) * 100)
let countdownTimer = null

function startCountdown() {
  stopCountdown()
  countdown.value = AUTO_CLOSE_SECONDS
  countdownTimer = setInterval(() => {
    countdown.value--
    if (countdown.value <= 0) {
      stopCountdown()
      emit('close')
    }
  }, 1000)
}

function stopCountdown() {
  clearInterval(countdownTimer)
  countdownTimer = null
}

function resetCountdown() {
  startCountdown()
}

function handleClose() {
  stopCountdown()
  emit('close')
}

watch(() => props.show, (val) => {
  if (val) startCountdown()
  else stopCountdown()
})

onUnmounted(stopCountdown)
</script>

<style scoped>
.payment-overlay {
  position: fixed;
  inset: 0;
  z-index: 9500;
  background: var(--bg-color);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
}

.payment-card {
  width: 100%;
  max-width: 720px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.payment-header {
  flex-shrink: 0;
  font-size: var(--font-xl);
  font-weight: 600;
  text-align: center;
  padding-top: 8px;
}
.payment-header strong {
  color: var(--accent-color);
  font-size: 1.8rem;
}

/* 二维码区域 */
.qr-area {
  flex: 1;
  min-height: 0;
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.qr-grid {
  display: flex;
  gap: 16px;
  align-items: center;
  justify-content: center;
  max-height: 100%;
  max-width: 100%;
}

/* 单码：居中最大化 */
.qr-grid.single .qr-wrapper {
  max-width: min(100%, 65vh);
}

/* 双码：各占一半，保证都能看到 */
.qr-grid:not(.single) .qr-wrapper {
  max-width: min(48%, 50vh);
}

.qr-wrapper {
  aspect-ratio: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 1;
  min-width: 0;
}

.qr-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: var(--radius-md);
}

.no-qr {
  padding: 2rem;
  color: var(--text-disabled);
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-md);
  text-align: center;
}
.no-qr p { margin: 0.5rem 0; }

.payment-footer {
  flex-shrink: 0;
  width: 100%;
  text-align: center;
  padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
}

.scan-tip {
  margin: 0 0 12px;
  color: var(--text-muted);
  font-size: var(--font-sm);
  line-height: 1.5;
}

.close-btn {
  font-weight: 700;
  font-size: var(--font-md);
}

.timer-bar {
  width: 100%;
  height: 3px;
  background: var(--border-color);
  border-radius: 2px;
  margin-bottom: 12px;
  overflow: hidden;
}
.timer-fill {
  height: 100%;
  background: var(--accent-color);
  border-radius: 2px;
  transition: width 1s linear;
}
.extend-btn {
  margin-top: 10px;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--font-sm);
  cursor: pointer;
  padding: 4px 12px;
  text-decoration: underline;
}
.extend-btn:hover { color: var(--accent-color); }

/* 过渡动画 */
.payment-fade-enter-active { transition: opacity 0.25s; }
.payment-fade-leave-active { transition: opacity 0.35s; }
.payment-fade-enter-from,
.payment-fade-leave-to { opacity: 0; }

/* 竖屏手机：双码改为上下排列 */
@media (max-width: 600px) and (orientation: portrait) {
  .qr-grid:not(.single) {
    flex-direction: column;
  }
  .qr-grid:not(.single) .qr-wrapper {
    max-width: min(80%, 35vh);
  }
}
</style>
