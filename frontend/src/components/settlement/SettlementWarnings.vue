<!--
  结算单对账警示条（spec §5.4）。

  「业务表加出来的数和账本对不上」意味着某笔账记错了，不能被稀释、不能折叠，
  必须逐条列在页面最上方。管理端把它提到页内首位（编辑区块之上、结算单之上），
  摊主端只读结算单里也用同一份渲染。
-->
<template>
  <n-alert
    v-if="warnings.length"
    type="error"
    :bordered="false"
    title="这张结算单和账本对不上，先别急着导出"
    class="warnings-block"
  >
    <p v-for="(w, i) in warnings" :key="i" class="warning-line">⚠ {{ w }}</p>
    <p class="warning-line muted">说明某笔账记错了，核对无误后再导出。</p>
  </n-alert>
</template>

<script setup lang="ts">
import { NAlert } from 'naive-ui'

defineProps<{ warnings: string[] }>()
</script>

<style scoped>
.warnings-block {
  margin-bottom: var(--space-lg);
}
.warning-line {
  margin: var(--space-xs) 0;
  line-height: 1.6;
  /* stylelint-disable-next-line declaration-property-value-keyword-no-deprecated -- 保留原关键字，不做行为变更 */
  word-break: break-word;
}
.warning-line.muted {
  color: var(--text-muted);
}
</style>
