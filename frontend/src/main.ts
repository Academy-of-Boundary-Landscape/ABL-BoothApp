// 入口：先拿到后端的实际地址，再加载应用。
//
// 后端首选 127.0.0.1:5140，被占时会换端口（src-tauri/src/server.rs）。client.ts / url.ts
// 在模块初始化时读 backendOrigin()，所以这一步必须在 import 它们之前完成——
// 应用本体放在 boot.ts 里动态导入。
import { initBackendOrigin } from './api/backendOrigin'

initBackendOrigin().then(() => import('./boot'))
