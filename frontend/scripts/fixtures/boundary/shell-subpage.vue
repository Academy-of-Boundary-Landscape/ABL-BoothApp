<!-- boundary-fixture-path: src/views/vendor/VendorOrders.vue -->
<template>
  <PageShell title="订单 expect-hit" width="full">
    <PageShell embedded width="full">不应命中</PageShell>
    <n-alert type="error" title="expect-hit">{{ store.error }}</n-alert>
    <n-alert type="error">表单校验失败，不应命中</n-alert>
    <n-alert type="error">{{ error }}（裸 error 是表单错误，不应命中）</n-alert>
    <!-- 注释里的 <table> 不应命中 -->
    <table class="expect-hit"></table>
  </PageShell>
</template>
