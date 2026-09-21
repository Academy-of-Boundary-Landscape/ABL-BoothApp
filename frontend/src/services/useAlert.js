// src/composables/useAlert.js

import { useAlertStore } from '@/stores/alertStore'

export function useAlert() {
  const alertStore = useAlertStore()

  /**
   * 触发全局弹窗的便捷函数
   * @param {string} message - 要显示的消息
   * @param {object} options - 可选配置 (title, type)
   */
  const showAlert = (message, options) => {
    alertStore.show(message, options)
  }

  // 你甚至可以创建一些快捷方式
  const showSuccess = (message, title = '成功') => {
    showAlert(message, { type: 'success', title })
  }

  const showError = (message, title = '错误') => {
    showAlert(message, { type: 'error', title })
  }

  return {
    showAlert,
    showSuccess,
    showError,
  }
}
