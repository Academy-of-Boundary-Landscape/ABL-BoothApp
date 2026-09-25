// composables/useDefaultPasswords.ts —— 两个全局密码是否仍是出厂默认值。
// 模块级单例：管理后台的提醒横幅读它，安全设置改完密码后调 refresh()，横幅随即消失。
import { ref } from 'vue'
import { api, unwrap, type Schemas } from '@/api/client'

const state = ref<Schemas['DefaultPasswordsResponse'] | null>(null)

async function refresh() {
  try {
    state.value = await unwrap(api.GET('/admin/default-passwords'))
  } catch {
    // 只是提醒，拿不到就不显示
    state.value = null
  }
}

export function useDefaultPasswords() {
  return { state, refresh }
}
