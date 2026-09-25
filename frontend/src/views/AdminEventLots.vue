<template>
  <PageShell embedded width="full">
    <!-- 说明：工作台子页是 embedded（外壳已有页头），PageShell 的 help 气泡不会渲染；
         共享的 helpContent 没有 event-lots 条目，本文件不能扩共享配置。这段说明保留在页面上，
         同时把「同货主」规则放进 LotDrawer 的字段说明。 -->
    <p class="page-hint">
      套装 = 从一组候选商品里按一个总价卖。选「这几样各 1 件凑齐」就是甲+乙合购； 选「任选 N
      件、可以拿同款」就是同一本也能买 3 本。
      <strong>候选商品必须属于同一个货主</strong>——替别的社团让价不是摊主能单方面决定的。
    </p>

    <SectionCard title="套装列表">
      <template #extra>
        <n-button type="primary" class="touch-target" @click="openCreate">新建套装</n-button>
      </template>

      <AsyncState
        :loading="store.isLoading"
        :error="store.error"
        :empty="!store.lots.length"
        loading-text="正在加载套装列表..."
      >
        <n-data-table
          :columns="columns"
          :data="store.lots"
          :bordered="false"
          :scroll-x="860"
          :row-key="(lot) => lot.id"
        />

        <template #empty>
          <EmptyState title="还没有套装" hint="配一个套装，顾客的购物车就会自动套用最省的那一种。">
            <template #action>
              <n-button type="primary" @click="openCreate">新建套装</n-button>
            </template>
          </EmptyState>
        </template>
      </AsyncState>
    </SectionCard>

    <LotDrawer
      v-model:show="drawerVisible"
      :event-id="props.id"
      :lot="editingLot"
      @saved="onSaved"
    />
  </PageShell>
</template>

<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref } from 'vue'
import { NButton, NDataTable, NSpace, type DataTableColumns } from 'naive-ui'
import { PageShell, SectionCard, AsyncState, EmptyState, Money } from '@/components/ui'
import { useFeedback } from '@/composables/useFeedback'
import { useLotStore } from '@/stores/lotStore'
import { useEventDetailStore } from '@/stores/eventDetailStore'
import LotDrawer from '@/components/event-prep/LotDrawer.vue'
import type { Schemas } from '@/api/client'

const props = defineProps<{ id: number }>()

const store = useLotStore()
const eventDetailStore = useEventDetailStore()
const fb = useFeedback()

const drawerVisible = ref(false)
const editingLot = ref<Schemas['LotResponse'] | null>(null)
const isBusy = ref(false)

function openCreate() {
  editingLot.value = null
  drawerVisible.value = true
}

function openEdit(lot: Schemas['LotResponse']) {
  editingLot.value = lot
  drawerVisible.value = true
}

function onSaved() {
  editingLot.value = null
}

function candidateNames(lot: Schemas['LotResponse']) {
  if (!lot.candidate_ids.length) return '（候选已被删除）'
  const byId = new Map(eventDetailStore.products.map((p): [number, string] => [p.id, p.name]))
  return lot.candidate_ids.map((id) => byId.get(id) || `#${id}`).join('、')
}

const columns = computed<DataTableColumns<Schemas['LotResponse']>>(() => [
  { title: '名称', key: 'name' },
  {
    title: '规则',
    key: 'rule',
    // 「任选 2 件 · 可同款」不许折成两行。
    minWidth: 150,
    ellipsis: { tooltip: true },
    // 与导入抽屉同一套说法（spec §4.3）：不可同款 = 从候选里挑 N 件不同的。
    render: (lot) =>
      lot.allow_repeat
        ? `任选 ${lot.pick_count} 件 · 可同款`
        : `任选 ${lot.pick_count} 件 · 各 1 件`,
  },
  {
    title: '总价',
    key: 'total_price',
    render: (lot) => h(Money, { value: lot.total_price }),
  },
  // 货主集中在一列，折行会把整张表撑高；给足宽度、超出省略。
  { title: '货主', key: 'owner_society_name', minWidth: 96, ellipsis: { tooltip: true } },
  {
    title: '候选商品',
    key: 'candidates',
    width: 320,
    render: (lot) => candidateNames(lot),
  },
  {
    title: '操作',
    key: 'actions',
    align: 'right',
    // 宽度要能容下「编辑 删除」两个小按钮；n-space 默认换行，显式关掉。
    width: 132,
    render: (lot) =>
      h(
        NSpace,
        { size: 'small', justify: 'end', wrap: false },
        {
          default: () => [
            h(
              NButton,
              { size: 'small', disabled: isBusy.value, onClick: () => openEdit(lot) },
              { default: () => '编辑' }
            ),
            h(
              NButton,
              {
                size: 'small',
                type: 'error',
                quaternary: true,
                disabled: isBusy.value,
                onClick: () => void handleDelete(lot),
              },
              { default: () => '删除' }
            ),
          ],
        }
      ),
  },
])

async function handleDelete(lot: Schemas['LotResponse']) {
  await fb.confirm({
    title: '确认删除',
    // 快照的存在是这句话成立的理由，不是安慰剧。
    content: `删除套装「${lot.name}」？已经下过的订单不受影响——它们存的是名字和价格的快照。`,
    positiveText: '确认删除',
    negativeText: '取消',
    danger: true,
    onConfirm: async () => {
      isBusy.value = true
      try {
        await store.deleteLot(props.id, lot.id)
        fb.success('套装已删除')
      } catch (error) {
        fb.error(error, '删除失败')
      } finally {
        isBusy.value = false
      }
    },
  })
}

onMounted(async () => {
  await eventDetailStore.fetchProductsForEvent(props.id)
  await store.fetchLots(props.id)
})
// 从 A 展会的套装页跳到 B 展会时，先渲染的是上一场的套装（候选名还会显示成 #id），
// 直到 fetchLots 返回。离开就清掉，别让旧展会的数据越界显示。
onUnmounted(() => {
  store.resetStore()
})
</script>

<style scoped>
/* 页头改 embedded 后，原副标题挪到内容区顶部；保证文字不丢。 */
.page-hint {
  margin: 0 0 var(--space-lg);
  color: var(--text-muted);
  font-size: var(--font-base);
  line-height: 1.6;
}

/* 手机上门禁要求可点区域 ≥ 44px。 */
@media (--phone) {
  .touch-target {
    min-height: 44px;
  }
}
</style>
