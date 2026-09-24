// src/composables/useAlert.ts

import { useAlertStore, type AlertOptions } from '@/stores/alertStore'

export function useAlert() {
  const alertStore = useAlertStore()

  /**
   * 触发全局弹窗的便捷函数
   * @param message - 要显示的消息
   * @param options - 可选配置 (title, type)
   */
  const showAlert = (message: string, options?: AlertOptions): void => {
    alertStore.show(message, options)
  }

  // 你甚至可以创建一些快捷方式
  const showSuccess = (message: string, title = '成功'): void => {
    showAlert(message, { type: 'success', title })
  }

  const showError = (message: string, title = '错误'): void => {
    showAlert(message, { type: 'error', title })
  }

  return {
    showAlert,
    showSuccess,
    showError,
  }
}
