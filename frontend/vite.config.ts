import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig(({ mode }) => {
  return {
    plugins: [vue()],
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url)),
      },
    },
    server: {
      //host : '0.0.0.0',
      proxy: {
        // 代理到新的静态文件路径
        '/static': {
          target: 'http://127.0.0.1:5140',
          changeOrigin: true,
        },
        '/uploads': {
          target: 'http://127.0.0.1:5140',
          changeOrigin: true,
        },
        '/api': {
          target: 'http://127.0.0.1:5140',
          changeOrigin: true,
        },
      },
      port: 5173,
      strictPort: true,
      host: true, // 加上这一行 在手机端测试时需要
    },
    build: {
      minify: 'esbuild',
    },
    test: {
      environment: 'jsdom',
      include: ['src/**/*.spec.ts'],
      globals: false,
    },
    esbuild: {
      // 在生产环境移除 console 和 debugger
      drop: mode === 'production' ? ['console', 'debugger'] : [],
    },
  }
})
