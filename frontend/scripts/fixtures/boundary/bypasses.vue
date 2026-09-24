<!-- 终审补的绕过写法：行尾带标记的每一行都必须被 check-ui-boundary 报出（--self-test 校验） -->
<template>
  <NModal :show="true" /><!-- expect-hit -->
</template>

<script setup lang="ts">
import * as naive from 'naive-ui' // expect-hit
import { NModal } from 'naive-ui/es/modal' // expect-hit
import { useModal, useNotification } from 'naive-ui' // expect-hit
import { createDiscreteApi } from 'naive-ui' // expect-hit
import { NButton } from 'naive-ui' // 不应命中

naive.useMessage()
globalThis.alert('x') // expect-hit
alert ('y') // expect-hit
self.confirm('z') // expect-hit
prompt('w') // expect-hit
const { innerWidth } = window // expect-hit
const mq = window.matchMedia('(max-width: 768px)') // expect-hit
const dark = window.matchMedia('(prefers-color-scheme: dark)') // 不应命中
function confirm(msg: string) {
  return msg
}
</script>

<style scoped>
/* stylelint-disable -- 整个文件都不管 */ /* expect-hit */
.a {
  /* stylelint-disable-line color-no-hex -- 行尾 */ /* expect-hit */
  color: #fff;
  /* stylelint-disable-next-line -- 没写规则名 */ /* expect-hit */
  color: #000;
  /* stylelint-disable-next-line color-no-hex -- 合法写法，不应命中 */
  color: #111;
}
/* stylelint-enable */ /* expect-hit */
</style>
