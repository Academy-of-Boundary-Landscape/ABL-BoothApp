import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  base: '/',

  // docs/superpowers/ 放的是内部工程 spec/plan，不是面向用户的文档，
  // 不该出现在文档站里；而且正文里的 <日期> <i64> <PathBuf> 这类尖括号内容
  // 会被 Vue 编译器当成未闭合的 HTML 标签，导致 docs:build 直接失败。
  srcExclude: ['**/superpowers/**'],

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
          { text: 'v1.2 更新', link: '/releases/v1.2.0' },
          { text: 'FAQ', link: '/faq/' },
          { text: '联系与支持', link: '/support/contact' }
        ],

        sidebar: {
      /* -------- Guide 教程 -------- */
      '/guide/': [
        {
          text: '快速上手',
          items: [
            { text: '5 分钟极速上手', link: '/guide/getting-started' },
            { text: '🆕 从 v1.1 升级到 v1.2', link: '/guide/upgrade-v1.2' }
          ]
        },
        {
          text: '使用指南',
          items: [
            { text: '推荐工作流', link: '/guide/workflow' },
            { text: '组网与连接', link: '/guide/network' },
            { text: '🆕 套装与优惠', link: '/guide/lots' },
            { text: '🆕 收摊与结算', link: '/guide/closing' },
            { text: '导出与复盘', link: '/guide/export' }
          ]
        },
        {
          text: '核心功能',
          items: [
            { text: 'AI 拍照识别', link: '/guide/vision-search' },
            { text: '🆕 条码扫描', link: '/guide/barcode-scan' },
            { text: '局域网 HTTPS', link: '/guide/lan-https' },
            { text: '自动更新', link: '/guide/auto-update' }
          ]
        },
        {
          text: '设计理念',
          items: [
            { text: '原子化与独立部署', link: '/guide/philosophy' },
            { text: '离线与支付问题', link: '/guide/why-offline' },
            { text: '适用边界', link: '/guide/boundary' }
          ]
        },
        {
          text: '版本说明',
          items: [
            { text: 'v1.2.0', link: '/releases/v1.2.0' },
            { text: 'v1.1.1', link: '/releases/v1.1.1' },
            { text: 'v1.1.0', link: '/releases/v1.1.0' }
          ]
        }
      ],
      '/releases/': [
        {
          text: '版本说明',
          items: [
            { text: 'v1.2.0', link: '/releases/v1.2.0' },
            { text: 'v1.1.1', link: '/releases/v1.1.1' },
            { text: 'v1.1.0', link: '/releases/v1.1.0' }
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
          { text: 'v1.2', link: '/en/releases/v1.2.0' },
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
                { text: '🆕 Upgrading from v1.1', link: '/en/guide/upgrade-v1.2' },
                { text: 'Recommended Workflow', link: '/en/guide/workflow' },
                { text: 'Networking & Connection', link: '/en/guide/network' },
                { text: '🆕 Bundles & Discounts', link: '/en/guide/lots' },
                { text: '🆕 Closing & Settlement', link: '/en/guide/closing' },
                { text: '🆕 Barcode Scanning', link: '/en/guide/barcode-scan' },
                { text: 'Export & Review', link: '/en/guide/export' }
              ]
            },
            {
              text: 'Release Notes',
              items: [{ text: 'v1.2.0', link: '/en/releases/v1.2.0' }]
            }
          ],
          '/en/releases/': [
            {
              text: 'Release Notes',
              items: [{ text: 'v1.2.0', link: '/en/releases/v1.2.0' }]
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
          { text: 'v1.2', link: '/ja/releases/v1.2.0' },
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
                { text: '🆕 v1.1 から v1.2 へのアップグレード', link: '/ja/guide/upgrade-v1.2' },
                { text: 'おすすめのワークフロー', link: '/ja/guide/workflow' },
                { text: 'ネットワークと接続', link: '/ja/guide/network' },
                { text: '🆕 セットと割引', link: '/ja/guide/lots' },
                { text: '🆕 撤収と精算', link: '/ja/guide/closing' },
                { text: '🆕 バーコードスキャン', link: '/ja/guide/barcode-scan' },
                { text: 'エクスポートと振り返り', link: '/ja/guide/export' }
              ]
            },
            {
              text: 'リリースノート',
              items: [{ text: 'v1.2.0', link: '/ja/releases/v1.2.0' }]
            }
          ],
          '/ja/releases/': [
            {
              text: 'リリースノート',
              items: [{ text: 'v1.2.0', link: '/ja/releases/v1.2.0' }]
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
                { text: '当日の運営', link: '/ja/faq/operation' },
                { text: 'トラブル対応', link: '/ja/faq/incidents' },
                { text: '画像表示', link: '/ja/faq/images-ui' },
                { text: 'データの安全と移行', link: '/ja/faq/data-migration' },
                { text: '応用テクニック', link: '/ja/faq/advanced' },
                { text: 'ハードウェアの推奨', link: '/ja/faq/hardware' },
                { text: 'コミュニティとオープンソース', link: '/ja/faq/community' }
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
