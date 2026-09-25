import { createRouter, createWebHistory, type RouteLocation } from 'vue-router'
import { useAuthStore } from '@/stores/authStore' // 导入 auth store
// 导入所有需要的布局和视图
import AdminLayout from '../views/AdminLayout.vue'
import AdminDashboard from '../views/AdminDashboard.vue'
import AdminMasterProducts from '../views/AdminMasterProducts.vue'
import AdminEventProducts from '../views/AdminEventProducts.vue'
import AdminEventLots from '../views/AdminEventLots.vue'
import AdminEventWorkbench from '../views/AdminEventWorkbench.vue'
import WorkbenchIndex from '../views/WorkbenchIndex.vue'
import AdminSocieties from '../views/AdminSocieties.vue'
import VendorEventSelection from '../views/VendorEventSelection.vue' // 【新增】导入新视图
import VendorShell from '../views/vendor/VendorShell.vue'
import VendorOrders from '../views/vendor/VendorOrders.vue'
import VendorInventory from '../views/vendor/VendorInventory.vue'
import VendorClosing from '../views/vendor/VendorClosing.vue'
import CustomerView from '../views/CustomerView.vue'
import EventPortalView from '../views/EventPortalView.vue'
import AdminEventOrders from '../views/AdminEventOrders.vue'
import LoginView from '../views/LoginView.vue'
import AdminEventStat from '../views/AdminEventStat.vue'
import AdminSettings from '../views/AdminSettings.vue'
import Help from '../views/Help.vue'
import NotFound from '../views/NotFound.vue'
import ServerError from '../views/ServerError.vue'

// beforeEach 里读取的 meta 字段；显式声明，免得 `to.meta.*` 退化成 unknown。
declare module 'vue-router' {
  interface RouteMeta {
    requiresAuth?: boolean
    role?: string
  }
}

// 工作台子页的 `id` 声明为 number；`props: true` 传的是路由参数原样的字符串，
// 严格比较（如导入抽屉里「排除本场」的 `e.id !== eventId`）会静默失效。
const workbenchIdProp = (to: RouteLocation) => ({ id: Number(to.params.id) })

const routes = [
  // --- 路由组 1: 管理后台 ---
  // 所有 /admin 开头的路径都会使用 AdminLayout 布局
  {
    path: '/admin',
    component: AdminLayout,
    meta: { requiresAuth: true, role: 'admin' },
    // 管理后台的所有子页面
    children: [
      {
        path: '', // /admin → 展会列表（原「控制台」已取消）
        redirect: { name: 'admin-events' },
      },
      {
        path: 'events',
        name: 'admin-events',
        component: AdminDashboard,
      },
      {
        // 展会工作台：外壳负责加载展会并 provide，子路由是展前 / 现场 / 收摊的具体页面。
        // 外壳**故意不命名**：vue-router 按父路由名跳转时不会匹配空路径子路由，
        // `<router-view>` 会是空的、按状态重定向也不发生。要进工作台就跳
        // `admin-event-workbench-index`（或直接用路径 `/admin/events/:id`）。
        path: 'events/:id',
        component: AdminEventWorkbench,
        children: [
          {
            // 按展会状态重定向到对应子页（spec §3.1）。
            path: '',
            name: 'admin-event-workbench-index',
            component: WorkbenchIndex,
          },
          {
            path: 'products',
            name: 'admin-event-products',
            component: AdminEventProducts,
            props: workbenchIdProp,
          },
          {
            path: 'lots',
            name: 'admin-event-lots',
            component: AdminEventLots,
            props: workbenchIdProp,
          },
          {
            path: 'orders',
            name: 'admin-event-orders',
            component: AdminEventOrders,
            props: workbenchIdProp,
          },
          {
            path: 'stats',
            name: 'admin-event-stats',
            component: AdminEventStat,
          },
          {
            path: 'settlement',
            name: 'admin-event-settlement',
            component: () => import('@/views/AdminEventSettlement.vue'),
            props: workbenchIdProp,
          },
        ],
      },
      {
        path: 'master-products',
        name: 'admin-master-products',
        component: AdminMasterProducts,
      },
      {
        path: 'societies',
        name: 'admin-societies',
        component: AdminSocieties,
      },
      {
        path: 'settings',
        name: 'admin-settings',
        component: AdminSettings,
      },
      {
        path: 'help',
        name: 'admin-help',
        component: Help,
      },
      {
        // 旧「关于」页并入设置页的「关于与更新」区块。
        path: 'about',
        redirect: { name: 'admin-settings', hash: '#about' },
      },
    ],
  },

  // --- 路由组 2: 摊主页面 ---
  // 这是一个独立的顶层路由，不使用 AdminLayout
  {
    path: '/vendor',
    name: 'vendor-select',
    component: VendorEventSelection,
  },

  // 摊主端外壳：订单 / 库存 / 收摊三个 tab 各是子路由；旧的 `/vendor/:id` URL 保留。
  {
    // 外壳同样不命名（理由见上面的展会工作台）：按名字跳这里不会走空路径子路由的重定向。
    path: '/vendor/:id', // :id 是展会 ID
    component: VendorShell,
    props: true,
    meta: { requiresAuth: true, role: 'vendor' },
    children: [
      {
        // 父路由的 :id 在子路由上仍存在；redirect 显式带过去。
        path: '',
        redirect: (to: RouteLocation) => ({ name: 'vendor-orders', params: { id: to.params.id } }),
      },
      {
        path: 'orders',
        name: 'vendor-orders',
        component: VendorOrders,
        props: true,
      },
      {
        path: 'inventory',
        name: 'vendor-inventory',
        component: VendorInventory,
        props: true,
      },
      {
        path: 'closing',
        name: 'vendor-closing',
        component: VendorClosing,
        props: true,
      },
    ],
  },

  // --- 路由组 3: 顾客点单页面 ---
  // 这也是一个独立的顶层路由，同样不使用 AdminLayout
  {
    path: '/events/:id/order', // 例如 /events/2/order
    name: 'customer-order',
    component: CustomerView,
    props: true, // 将 :id 作为 prop 传递给 CustomerView
  },
  {
    path: '/',
    name: 'customer',
    component: EventPortalView,
  },
  {
    path: '/login/:role', // 动态角色：/login/admin 或 /login/vendor
    name: 'login',
    component: LoginView,
    props: true, // 将 :role 作为 prop 传给 LoginView
  },
  // --- 错误页 ---
  {
    path: '/error/500',
    name: 'server-error',
    component: ServerError,
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: NotFound,
  },
]
const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
})

router.beforeEach((to, from, next) => {
  const authStore = useAuthStore()

  const requiresAuth = to.meta.requiresAuth
  const requiredRole = to.meta.role

  if (requiresAuth) {
    let hasPermission = false
    if (requiredRole === 'admin' && authStore.isAdmin) {
      hasPermission = true
    }

    if (requiredRole === 'vendor') {
      // `:id` 只会有单个值；params 的类型允许 string[]，先收窄再交给 store。
      const eventId = Array.isArray(to.params.id) ? to.params.id[0] : to.params.id
      if (authStore.canAccessVendorPage(eventId)) {
        hasPermission = true
      }
    }

    if (hasPermission) {
      next() // 权限通过，放行
    } else {
      next({
        name: 'login',
        params: { role: requiredRole || 'vendor' }, // 提供一个默认角色
        query: { redirect: to.fullPath, eventId: to.params.id },
      })
    }
  } else {
    next() // 页面不需要认证，直接放行
  }
})
export default router
