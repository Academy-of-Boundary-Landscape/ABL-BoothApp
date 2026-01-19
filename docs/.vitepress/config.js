import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  base :'/',
  title: 'BoothKernel',
  description: '现代的同人摊主点单与记账系统',
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }],
    // 可选：如果你有 svg 版
    // ['link', { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
    // 可选：苹果设备
    // ['link', { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' }],
  ],

  themeConfig: {
    /* =========================
     * 顶部导航（Nav）
     * ========================= */
    nav: [
      { text: '主页', link: '/' },
      { text: '快速上手', link: '/guide/getting-started' },
      { text: 'FAQ', link: '/faq/' },
      { text: '联系与支持', link: '/support/contact' }
    ],

    /* =========================
     * 侧边栏（Sidebar）
     * 按路径分别定义，避免混在一起
     * ========================= */
    sidebar: {
      /* -------- Guide 教程 -------- */
      '/guide/': [
        {
          text: '快速上手',
          items: [
            {
              text: '5 分钟极速上手',
              link: '/guide/getting-started'
            }
          ]
        },
        {
          text: '使用指南',
          items: [
            { text: '组网与连接', link: '/guide/network' },
            { text: '工作流说明', link: '/guide/workflow' },
            { text: '导出与复盘', link: '/guide/export' }
          ]
        }
      ],

      /* -------- FAQ -------- */
      '/faq/': [
        {
          text: 'FAQ 总览',
          items: [
            { text: '常见问题总览', link: '/faq/' }
          ]
        },
        {
          text: '常见问题分类',
          items: [
            { text: '网络连接', link: '/faq/network' },
            { text: '现场运营', link: '/faq/operation' },
            { text: '突发状况', link: '/faq/incidents' },
            { text: '图片显示', link: '/faq/images-ui' },
            { text: '数据安全与迁移', link: '/faq/data-migration' },
            { text: '高级技巧', link: '/faq/advanced' },
            { text: '硬件建议', link: '/faq/hardware' },
            { text: '社区与开源', link: '/faq/community' }
          ]
        }
      ],
      '/support/': [
      { text: '支持', items: [{ text: '联系与支持', link: '/support/contact' }] }
    ]
    },

    /* =========================
     * 其它推荐设置
     * ========================= */

    outline: {
      level: [2, 3],
      label: '本页内容'
    },

    socialLinks: [
      {
        icon: 'github',
        link: 'https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp'
      }
    ]
  }
})
