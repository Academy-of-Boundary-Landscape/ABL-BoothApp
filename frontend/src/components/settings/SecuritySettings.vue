<template>
  <SectionCard title="安全设置" collapsible v-model:collapsed="collapsed">
    <div class="settings-grid">
      <div class="settings-card">
        <div class="settings-title">管理员密码</div>
        <n-form :model="adminForm" label-placement="top">
          <n-form-item label="旧密码">
            <n-input
              v-model:value="adminForm.oldPassword"
              type="password"
              show-password-on="click"
              placeholder="请输入旧密码"
            />
          </n-form-item>
          <n-form-item label="新密码 (至少 4 位)">
            <n-input
              v-model:value="adminForm.newPassword"
              type="password"
              show-password-on="click"
              placeholder="请输入新密码"
            />
          </n-form-item>
          <n-space justify="end">
            <n-button type="primary" :loading="adminSaving" @click="updateAdminPassword"
              >保存</n-button
            >
          </n-space>
        </n-form>
        <n-alert v-if="adminMessage" :type="adminMessage.type" :bordered="false" class="mt-8">{{
          adminMessage.text
        }}</n-alert>
      </div>

      <div class="settings-card">
        <div class="settings-title">默认摊主密码（未配置摊主密码时采用）</div>
        <n-form :model="vendorForm" label-placement="top">
          <n-form-item label="新密码 (至少 4 位)">
            <n-input
              v-model:value="vendorForm.newPassword"
              type="password"
              show-password-on="click"
              placeholder="请输入新密码"
            />
          </n-form-item>
          <n-space justify="end">
            <n-button type="primary" :loading="vendorSaving" @click="updateVendorPassword"
              >保存</n-button
            >
          </n-space>
        </n-form>
        <n-alert v-if="vendorMessage" :type="vendorMessage.type" :bordered="false" class="mt-8">{{
          vendorMessage.text
        }}</n-alert>
      </div>
    </div>
  </SectionCard>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NAlert, NButton, NForm, NFormItem, NInput, NSpace } from 'naive-ui'
import { SectionCard } from '@/components/ui'
import { api, unwrap, errorMessage } from '@/api/client'
import { useAuthStore } from '@/stores/authStore'

const authStore = useAuthStore()
const collapsed = ref(false)
const adminForm = ref({ oldPassword: '', newPassword: '' })
const vendorForm = ref({ newPassword: '' })
const adminSaving = ref(false)
const vendorSaving = ref(false)
const adminMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)
const vendorMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

async function updateAdminPassword() {
  const newPassword = adminForm.value.newPassword
  if (!adminForm.value.oldPassword) {
    adminMessage.value = { type: 'error', text: '请输入旧密码' }
    return
  }
  if (newPassword.length < 4) {
    adminMessage.value = { type: 'error', text: '新密码至少 4 位' }
    return
  }
  adminSaving.value = true
  adminMessage.value = null
  try {
    await unwrap(
      api.PUT('/admin/password', {
        body: {
          oldPassword: adminForm.value.oldPassword,
          newPassword,
        },
      })
    )
    adminMessage.value = { type: 'success', text: '管理员密码已更新' }
    adminForm.value = { oldPassword: '', newPassword: '' }
    // 密码改了需要重新登录
    await authStore.login(newPassword, 'admin')
  } catch (e) {
    adminMessage.value = {
      type: 'error',
      text: errorMessage(e, '修改失败'),
    }
  } finally {
    adminSaving.value = false
  }
}

async function updateVendorPassword() {
  if (vendorForm.value.newPassword.length < 4) {
    vendorMessage.value = { type: 'error', text: '新密码至少 4 位' }
    return
  }
  vendorSaving.value = true
  vendorMessage.value = null
  try {
    await unwrap(
      api.PUT('/admin/vendor-default-password', {
        body: { newPassword: vendorForm.value.newPassword },
      })
    )
    vendorMessage.value = { type: 'success', text: '默认摊主密码已更新' }
    vendorForm.value = { newPassword: '' }
  } catch (e) {
    vendorMessage.value = {
      type: 'error',
      text: errorMessage(e, '修改失败'),
    }
  } finally {
    vendorSaving.value = false
  }
}
</script>

<style scoped>
.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-xl);
}

@media (--phone) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
}

.settings-card {
  padding: var(--space-lg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-color);
}

.settings-title {
  font-weight: var(--weight-bold);
  margin-bottom: var(--space-lg);
  font-size: var(--font-md);
}

.mt-8 {
  margin-top: var(--space-sm);
}
</style>
