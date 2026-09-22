<template>
  <AppModal :show="show" @close="$emit('cancel')">
    <template #header><h3>记录收款方式</h3></template>
    <template #body>
      <n-radio-group v-model:value="channel" name="payment-channel">
        <n-space>
          <n-radio v-for="c in CHANNELS" :key="c" :value="c">{{ c }}</n-radio>
        </n-space>
      </n-radio-group>

      <!-- spec 第 11 节的不可破坏项：不能让复式记账制造出「钱已到账」的错觉。
           系统始终不知道顾客有没有真付，摊主点的是「我看到到账提示了」。 -->
      <p class="disclosure">这只是记账。请先确认手机上真的收到了到账提示，再点确认。</p>
    </template>
    <template #footer>
      <n-space>
        <n-button @click="$emit('cancel')">取消</n-button>
        <n-button type="primary" @click="handleConfirm">确认</n-button>
      </n-space>
    </template>
  </AppModal>
</template>

<script setup>
import { ref, watch } from 'vue'
import { NRadioGroup, NRadio, NSpace, NButton } from 'naive-ui'
import AppModal from '@/components/shared/AppModal.vue'

// 现场一场展会里收款渠道基本不变，上次选的记 localStorage 做默认值。
const CHANNELS = ['现金', '微信', '支付宝']
const CHANNEL_STORAGE_KEY = 'last_payment_channel'

const props = defineProps({
  show: { type: Boolean, default: false },
})
const emit = defineEmits(['confirm', 'cancel'])

const channel = ref('微信')

watch(
  () => props.show,
  (val) => {
    if (!val) return
    const saved = localStorage.getItem(CHANNEL_STORAGE_KEY)
    channel.value = CHANNELS.includes(saved) ? saved : '微信'
  }
)

function handleConfirm() {
  localStorage.setItem(CHANNEL_STORAGE_KEY, channel.value)
  emit('confirm', channel.value)
}
</script>

<style scoped>
.disclosure {
  margin: 16px 0 0;
  font-size: var(--font-sm);
  line-height: 1.5;
  color: var(--text-muted);
}
</style>
