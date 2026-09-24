<template>
  <div class="page-header">legacy-class</div>
  <n-modal v-model:show="show">naive-feedback (template)</n-modal>
  <!-- ui-boundary-ignore: 豁免用例：这一行的 table-wrapper 不应被报出 -->
  <div class="table-wrapper">exempt legacy-class</div>
</template>

<script setup lang="ts">
import { useMessage } from 'naive-ui'
import AppModal from '@/components/shared/AppModal.vue'
import { ref } from 'vue'

const show = ref(false)

alert('native-dialog')
window.confirm('native-dialog')

fb.confirm('成员调用，不应命中')
foo.alert('成员调用，不应命中')

const width = window.innerWidth
</script>
