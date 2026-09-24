// src/stores/alertStore.ts

import { defineStore } from 'pinia'
import { ref } from 'vue'

/** `show` 的可选配置。 */
export interface AlertOptions {
  title?: string
  type?: string
}

export const useAlertStore = defineStore('alert', () => {
  // State: 弹窗的状态
  const isVisible = ref(false)
  const message = ref('')
  const title = ref('提示')
  const type = ref('info') // 'info', 'success', 'warning', 'error'

  // Actions: 控制弹窗的方法

  /**
   * 显示弹窗
   * @param msg - 要显示的消息
   * @param options - 可选配置
   * @param options.title - 弹窗标题
   * @param options.type - 弹窗类型 ('info', 'success', 'warning', 'error')
   */
  function show(msg: string, options: AlertOptions = {}): void {
    message.value = msg
    title.value = options.title || '提示'
    type.value = options.type || 'info'
    isVisible.value = true
  }

  function hide(): void {
    isVisible.value = false
    // 重置为默认值，以防下次调用时残留
    message.value = ''
    title.value = '提示'
    type.value = 'info'
  }

  return { isVisible, message, title, type, show, hide }
})
