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
        :empty="!store.societies.length"
        loading-text="正在加载社团列表..."
      >
        <n-data-table
          :columns="columns"
          :data="store.societies"
          :row-key="(row) => row.id"
          :scroll-x="560"
          size="small"
        />

        <template #empty>
          <EmptyState
            icon="🏷️"
            title="暂无社团"
            desc="社团是货主的单位，商品上架时会按社团归属记账。"
            hint="在上方输入名称并点「新建」"
          />
        </template>
      </AsyncState>
    </main>
  </PageShell>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import { NButton, NDataTable, NInput, NTag, type DataTableColumns } from 'naive-ui'
import { PageShell, AsyncState, EmptyState } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useSocietyStore } from '@/stores/societyStore'
import type { Schemas } from '@/api/client'

const store = useSocietyStore()
const fb = useFeedback()

const newName = ref('')
const isCreating = ref(false)
const isBusy = ref(false)

const columns: DataTableColumns<Schemas['Society']> = [
  { title: '名字', key: 'name', minWidth: 160 },
  {
    title: '是否本社团',
    key: 'is_home',
    width: 140,
    render: (society) =>
      society.is_home
        ? h(NTag, { type: 'success', size: 'small', round: true }, { default: () => '本社团' })
        : h('span', { class: 'muted' }, '—'),
  },
  {
    title: '操作',
    key: 'actions',
    width: 220,
    align: 'right',
    render: (society) =>
      society.is_home
        ? null
        : h('div', { class: 'row-actions' }, [
            h(
              NButton,
              { size: 'small', disabled: isBusy.value, onClick: () => handleSetHome(society) },
              { default: () => '设为本社团' }
            ),
            h(
              NButton,
              {
                size: 'small',
                type: 'error',
                quaternary: true,
                disabled: isBusy.value,
                onClick: () => handleDelete(society),
              },
              { default: () => '删除' }
            ),
          ]),
  },
]

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

.row-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
}

.muted {
  color: var(--text-disabled);
}

@media (--phone) {
  .create-row {
    flex-direction: column;
  }
}
</style>
