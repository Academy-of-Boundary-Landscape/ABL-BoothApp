<template>
  <PageShell
    title="社团管理"
    subtitle="社团是货主的单位。每个商品都归属一个社团，「本社团」有且只有一个。"
    width="content"
  >
    <main class="page-body">
      <div class="create-row">
        <n-input
          v-model:value="newName"
          placeholder="新社团名称"
          clearable
          @keyup.enter="handleCreate"
        />
        <n-button type="primary" :disabled="isCreating" @click="handleCreate">
          {{ isCreating ? '新建中...' : '新建' }}
        </n-button>
      </div>

      <AsyncState
        :loading="store.isLoading"
        :error="store.error"
        loading-text="正在加载社团列表..."
      >
        <div class="table-scroll">
          <table class="society-table">
            <thead>
              <tr>
                <th>名字</th>
                <th>是否本社团</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="society in store.societies" :key="society.id">
                <td>{{ society.name }}</td>
                <td>
                  <n-tag v-if="society.is_home" type="success" size="small" round>本社团</n-tag>
                  <span v-else class="muted">—</span>
                </td>
                <td>
                  <n-space size="small" justify="end">
                    <n-button
                      v-if="!society.is_home"
                      size="small"
                      @click="handleSetHome(society)"
                      :disabled="isBusy"
                      >设为本社团</n-button
                    >
                    <n-button
                      v-if="!society.is_home"
                      size="small"
                      type="error"
                      quaternary
                      @click="handleDelete(society)"
                      :disabled="isBusy"
                      >删除</n-button
                    >
                  </n-space>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </AsyncState>
    </main>
  </PageShell>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NInput, NButton, NSpace, NTag } from 'naive-ui'
import { PageShell, AsyncState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useSocietyStore } from '@/stores/societyStore'
import type { Schemas } from '@/api/client'

const store = useSocietyStore()
const fb = useFeedback()

const newName = ref('')
const isCreating = ref(false)
const isBusy = ref(false)

async function handleCreate() {
  const name = newName.value.trim()
  if (!name) {
    fb.warning('请输入社团名称')
    return
  }
  isCreating.value = true
  try {
    await store.createSociety(name)
    newName.value = ''
    fb.success('社团已新建')
  } catch (error) {
    fb.error(error, '新建失败')
  } finally {
    isCreating.value = false
  }
}

async function handleSetHome(society: Schemas['Society']) {
  isBusy.value = true
  try {
    await store.setHomeSociety(society.id)
    fb.success(`「${society.name}」已设为本社团`)
  } catch (error) {
    fb.error(error, '操作失败')
  } finally {
    isBusy.value = false
  }
}

async function handleDelete(society: Schemas['Society']) {
  await fb.confirm({
    title: '确认删除',
    content: `确定要删除社团「${society.name}」吗？还有商品归属它时不能删除。`,
    positiveText: '确认删除',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      isBusy.value = true
      try {
        await store.deleteSociety(society.id)
        fb.success('社团已删除')
      } catch (error) {
        fb.error(error, '删除失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

onMounted(() => {
  store.fetchSocieties()
})
</script>

<style scoped>
.create-row {
  display: flex;
  gap: var(--space-md);
  margin-bottom: var(--space-xl);
}

.society-table {
  width: 100%;
  border-collapse: collapse;
  border-spacing: 0;
  text-align: left;
  font-size: var(--font-base);
}

.society-table th {
  padding: var(--space-md) var(--space-lg);
  background-color: var(--card-bg-color);
  color: var(--primary-text-color);
  font-weight: var(--weight-bold);
  border-bottom: 2px solid var(--accent-color);
  white-space: nowrap;
}

.society-table td {
  padding: var(--space-md) var(--space-lg);
  border-bottom: 1px solid var(--border-color);
  color: var(--text-placeholder);
  vertical-align: middle;
}

.society-table tbody tr:hover {
  background-color: var(--accent-color-light);
}

.society-table th:last-child,
.society-table td:last-child {
  text-align: right;
}

.muted {
  color: var(--text-disabled);
}
</style>
