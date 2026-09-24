import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import postcssCustomMedia from 'postcss-custom-media'
import postcssGlobalData from '@csstools/postcss-global-data'

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
    css: {
      // styles/media.css 定义 custom media；global-data 把定义注入每个文件
      // （SFC 的 <style> 也适用），postcss-custom-media 负责展开成真实媒体查询。
      postcss: {
        plugins: [
          postcssGlobalData({
            files: [fileURLToPath(new URL('./src/styles/media.css', import.meta.url))],
          }),
          postcssCustomMedia(),
        ],
      },
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
