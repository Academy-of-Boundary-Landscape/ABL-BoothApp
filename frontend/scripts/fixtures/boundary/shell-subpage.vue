<!-- boundary-fixture-path: src/views/vendor/VendorOrders.vue -->
<template>
  <PageShell title="订单" width="full">
    expect-hit
    <PageShell embedded width="full">不应命中</PageShell>
    <n-alert type="error" :bordered="false">{{ store.error }}</n-alert> expect-hit
    <n-alert type="error">表单校验失败，不应命中</n-alert>
    <table class="x">
      expect-hit
    </table>
  </PageShell>
</template>
<!-- 注释里的 <table> 不应命中 -->
<n-alert type="error">{{ error }}</n-alert>
<!-- 裸 error 是表单错误，不应命中 -->
