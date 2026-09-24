// 1. 确保从 'vue' 导入 createApp
import { createApp } from 'vue'

// 2. 确保从 'pinia' 导入 createPinia
import { createPinia } from 'pinia'

// 3. 导入你的根组件、路由和全局样式
import App from './App.vue'
import router from './router'
import './assets/main.css'

// 4. 创建 Vue 应用实例
const app = createApp(App)

// 5. 使用 Pinia 和路由
app.use(createPinia())
app.use(router)

// 6. 挂载应用
app.mount('#app')
