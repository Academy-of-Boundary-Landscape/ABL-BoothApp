<template>
  <div class="fixture">stylelint 豁免样本（由 src/gates.spec.ts 断言）</div>
</template>

<!-- 无理由的 stylelint-disable-next-line 必须报 descriptionless disable -->
<style scoped>
.a {
  /* stylelint-disable-next-line color-no-hex */
  color: #fff;
}
</style>
