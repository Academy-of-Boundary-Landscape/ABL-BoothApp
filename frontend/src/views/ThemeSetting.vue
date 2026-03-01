<template>
  <div class="theme-setting-container">
    <n-grid x-gap="24" y-gap="24" cols="1 s:1 m:2" responsive="screen">

      <!-- 左侧：设置面板 -->
      <n-grid-item>
        <n-card title="主题偏好" size="medium">
          <n-space vertical size="large">

            <!-- 1. 模式切换 -->
            <div class="setting-item">
              <div class="label">
                <span class="title">界面模式</span>
                <span class="desc">切换深色或浅色外观</span>
              </div>
              <n-switch v-model:value="themeStore.isDark" size="large">
                <template #checked-icon>
                  <n-icon :component="Moon" />
                </template>
                <template #unchecked-icon>
                  <n-icon :component="Sunny" color="#f59e0b" />
                </template>
              </n-switch>
            </div>

            <n-divider />

            <!-- 2. 颜色选择 -->
            <div class="setting-item-vertical">
              <div class="label-row">
                <span class="title">品牌主色</span>
                <n-button text type="primary" size="small" @click="themeStore.resetColor">
                  恢复默认
                </n-button>
              </div>

              <!-- 预设色板 -->
              <div class="color-swatches">
                <n-tooltip
                  v-for="swatch in presetColors"
                  :key="swatch.color"
                  placement="top"
                >
                  <template #trigger>
                    <div
                      class="swatch"
                      :style="{ backgroundColor: swatch.color }"
                      :class="{ active: themeStore.customPrimaryColor === swatch.color || (!themeStore.customPrimaryColor && isDefaultColor(swatch)) }"
                      @click="themeStore.customPrimaryColor = swatch.color"
                    >
                      <n-icon
                        v-if="themeStore.customPrimaryColor === swatch.color || (!themeStore.customPrimaryColor && isDefaultColor(swatch))"
                        :component="Checkmark"
                      />
                    </div>
                  </template>
                  <span>{{ swatch.label }}</span>
                </n-tooltip>
              </div>

              <!-- 自定义取色器 -->
              <n-color-picker
                v-model:value="themeStore.customPrimaryColor"
                :show-alpha="false"
                :actions="['confirm']"
                class="mt-2"
              />
            </div>

          </n-space>
        </n-card>
      </n-grid-item>

      <!-- 右侧：实时预览 -->
      <n-grid-item>
        <n-card title="效果预览" size="medium" class="preview-card">
          <n-space vertical size="large">

            <!-- 按钮展示 -->
            <n-space>
              <n-button type="primary">主要按钮</n-button>
              <n-button secondary type="primary">次要按钮</n-button>
              <n-button ghost type="primary">边框按钮</n-button>
              <n-button>默认按钮</n-button>
            </n-space>

            <!-- 标签展示 -->
            <n-space>
              <n-tag type="primary">标签 One</n-tag>
              <n-tag type="success">成功</n-tag>
              <n-tag type="warning">警告</n-tag>
              <n-tag type="error">错误</n-tag>
            </n-space>

            <!-- 输入框展示 -->
            <n-input placeholder="聚焦这里查看边框颜色..." />

            <!-- 提示框展示 -->
            <n-alert title="主题色应用成功" type="info">
              当前的组件颜色会自动根据你选择的主色调进行计算，包括 Hover 和 Pressed 状态。
            </n-alert>

            <!-- 卡片展示 -->
            <div class="custom-box">
              <p>这是一个使用了 CSS 变量的自定义 Div</p>
              <code>color: var(--accent-color)</code>
            </div>

          </n-space>
        </n-card>
      </n-grid-item>
    </n-grid>
  </div>
</template>

<script setup>
import {
  NCard, NSpace, NSwitch, NDivider, NButton, NColorPicker,
  NGrid, NGridItem, NIcon, NTag, NInput, NAlert, NTooltip
} from 'naive-ui'
import { Sunny, Moon, Checkmark } from '@vicons/ionicons5'
import { useThemeStore } from '@/stores/themeStore'

const themeStore = useThemeStore()

// 预设主色：带名称，悬停时通过 title 展示
const presetColors = [
  { color: '#8B0012', label: '北大红' },   // 当前主题默认主色
  { color: '#FFA6A6', label: '少女粉' },
  { color: '#E9BA00', label: '奶龙黄' },
  { color: '#007ACC', label: 'VSCode蓝' },
  { color: '#62FFF4', label: '默认青（旧）' },
  { color: '#660874', label: '清华紫' },
]

const isDefaultColor = (swatch) => {
  return swatch.color === themeStore.currentBaseTheme.primary.base
}
</script>

<style scoped>
.theme-setting-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.setting-item-vertical {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-weight: 600;
  font-size: 16px;
  color: var(--primary-text-color);
}

.desc {
  font-size: 12px;
  color: var(--secondary-text-color);
  margin-top: 4px;
}

/* 色板样式 */
.color-swatches {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.swatch {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  transition: transform 0.2s, box-shadow 0.2s;
  border: 2px solid transparent;
}

.swatch:hover {
  transform: scale(1.1);
}

.swatch.active {
  transform: scale(1.1);
  box-shadow: 0 0 0 2px var(--card-bg-color), 0 0 0 4px var(--primary-text-color);
}

.mt-2 {
  margin-top: 8px;
}

/* 自定义 CSS 变量测试框 */
.custom-box {
  padding: 16px;
  border: 1px dashed var(--accent-color);
  background-color: var(--accent-color-light);
  color: var(--accent-color-dark);
  border-radius: 8px;
  text-align: center;
  font-weight: bold;
}
</style>
