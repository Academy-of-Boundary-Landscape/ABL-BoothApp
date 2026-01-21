import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  base: '/',
  
  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      title: 'BoothKernel',
      description: '现代的同人摊主点单与记账系统',
      
      themeConfig: {
        nav: [
          { text: '主页', link: '/' },
          { text: '快速上手', link: '/guide/getting-started' },
          { text: 'FAQ', link: '/faq/' },
          { text: '联系与支持', link: '/support/contact' }
        ],

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
          text: '设计理念',
          items: [
            {
              text: '为什么选择离线方案？',
              link: '/guide/why-offline'
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
    },
    
    en: {
      label: 'English',
      lang: 'en-US',
      title: 'BoothKernel',
      description: 'Modern POS System for Doujin Events',
      
      themeConfig: {
        nav: [
          { text: 'Home', link: '/en/' },
          { text: 'Getting Started', link: '/en/guide/getting-started' },
          { text: 'FAQ', link: '/en/faq/' },
          { text: 'Support', link: '/en/support/contact' }
        ],

        sidebar: {
          /* -------- Guide -------- */
          '/en/guide/': [
            {
              text: 'Getting Started',
              items: [
                {
                  text: '5-Minute Quick Start',
                  link: '/en/guide/getting-started'
                }
              ]
            },
            {
              text: 'User Guide',
              items: [
                { text: 'Network Setup', link: '/en/guide/network' },
                { text: 'Workflow Guide', link: '/en/guide/workflow' },
                { text: 'Export & Review', link: '/en/guide/export' }
              ]
            }
          ],

          /* -------- FAQ -------- */
          '/en/faq/': [
            {
              text: 'FAQ Overview',
              items: [
                { text: 'FAQ Index', link: '/en/faq/' }
              ]
            },
            {
              text: 'Categories',
              items: [
                { text: 'Network Connection', link: '/en/faq/network' },
                { text: 'Operations', link: '/en/faq/operation' },
                { text: 'Incidents', link: '/en/faq/incidents' },
                { text: 'Images & UI', link: '/en/faq/images-ui' },
                { text: 'Data Migration', link: '/en/faq/data-migration' },
                { text: 'Advanced Tips', link: '/en/faq/advanced' },
                { text: 'Hardware Recommendations', link: '/en/faq/hardware' },
                { text: 'Community', link: '/en/faq/community' }
              ]
            }
          ],
          
          '/en/support/': [
            { text: 'Support', items: [{ text: 'Contact & Support', link: '/en/support/contact' }] }
          ]
        },
        
        outline: {
          level: [2, 3],
          label: 'On This Page'
        },

        socialLinks: [
          {
            icon: 'github',
            link: 'https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp'
          }
        ]
      }
    }
    ,
    ja: {
      label: '日本語',
      lang: 'ja-JP',
      title: 'BoothKernel',
      description: '同人イベント向けの近代的な出店管理システム',
      
      themeConfig: {
        nav: [
          { text: 'ホーム', link: '/ja/' },
          { text: 'クイックスタート', link: '/ja/guide/getting-started' },
          { text: 'FAQ', link: '/ja/faq/' },
          { text: 'サポート', link: '/ja/support/contact' }
        ],

        sidebar: {
          /* -------- Guide -------- */
          '/ja/guide/': [
            {
              text: 'クイックスタート',
              items: [
                {
                  text: '5分で始める',
                  link: '/ja/guide/getting-started'
                }
              ]
            },
            {
              text: 'ユーザーガイド',
              items: [
                { text: 'ネットワーク設定', link: '/ja/guide/network' },
                { text: 'ワークフロー', link: '/ja/guide/workflow' },
                { text: 'エクスポートとレビュー', link: '/ja/guide/export' }
              ]
            }
          ],

          /* -------- FAQ -------- */
          '/ja/faq/': [
            {
              text: 'FAQ 概要',
              items: [
                { text: 'FAQ インデックス', link: '/ja/faq/' }
              ]
            },
            {
              text: 'カテゴリ',
              items: [
                { text: 'ネットワーク接続', link: '/ja/faq/network' },
                { text: '運用', link: '/ja/faq/operation' },
                { text: 'トラブル対応', link: '/ja/faq/incidents' },
                { text: '画像とUI', link: '/ja/faq/images-ui' },
                { text: 'データ移行', link: '/ja/faq/data-migration' },
                { text: '上級テクニック', link: '/ja/faq/advanced' },
                { text: 'ハードウェア推奨', link: '/ja/faq/hardware' },
                { text: 'コミュニティ', link: '/ja/faq/community' }
              ]
            }
          ],
          
          '/ja/support/': [
            { text: 'サポート', items: [{ text: 'お問い合わせ', link: '/ja/support/contact' }] }
          ]
        },
        
        outline: {
          level: [2, 3],
          label: 'このページの内容'
        },

        socialLinks: [
          {
            icon: 'github',
            link: 'https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp'
          }
        ]
      }
    }
  },
  
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }],
  ]
})
