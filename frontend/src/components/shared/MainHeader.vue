<template>
  <div class="app-header-container">
    <header class="app-header" :class="{ collapsed: isCollapsed }">
      <div class="app-header-left">
        <span class="app-logo">页面导航栏</span>

        <!-- 新增：移动端快捷导航 (仅在菜单未打开时显示) -->
        <div class="mobile-shortcuts" v-if="!isOpen">
          <RouterLink to="/" class="shortcut-btn" active-class="shortcut-btn--active">
            <NIcon><PersonOutline /></NIcon>
            <!-- 如果想用汉字，可以把 NIcon 换成 <span>户</span> -->
          </RouterLink>
          <RouterLink to="/vendor" class="shortcut-btn" active-class="shortcut-btn--active">
            <NIcon><StorefrontOutline /></NIcon>
            <!-- <span>摊</span> -->
          </RouterLink>
          <RouterLink to="/admin" class="shortcut-btn" active-class="shortcut-btn--active">
            <NIcon><SettingsOutline /></NIcon>
            <!-- <span>管</span> -->
          </RouterLink>
        </div>

        <button class="menu-toggle" @click="isOpen = !isOpen" aria-label="切换导航">
          <span class="menu-icon" :class="{ open: isOpen }"></span>
        </button>
      </div>

      <!-- 完整的导航菜单 (PC端常显，移动端展开显示) -->
      <nav class="app-header-nav" :class="{ open: isOpen }">
        <RouterLink
          to="/"
          class="nav-link"
          active-class="nav-link--active"
          exact-active-class="nav-link--active"
          @click="closeIfMobile"
        >
          <NIcon class="nav-icon-desktop"><PersonOutline /></NIcon> 点单页面
        </RouterLink>
        <RouterLink
          to="/vendor"
          class="nav-link"
          active-class="nav-link--active"
          @click="closeIfMobile"
        >
          <NIcon class="nav-icon-desktop"><StorefrontOutline /></NIcon> 摊主页面
        </RouterLink>
        <RouterLink
          to="/admin"
          class="nav-link"
          active-class="nav-link--active"
          @click="closeIfMobile"
        >
          <NIcon class="nav-icon-desktop"><SettingsOutline /></NIcon> 管理员页面
        </RouterLink>
      </nav>

      <button class="collapse-toggle" @click="toggleCollapse" aria-label="收起导航">
        <NIcon class="chevron-icon" :class="{ closed: isCollapsed }">
          <ChevronDown />
        </NIcon>
      </button>
    </header>

    <button
      v-if="isCollapsed"
      class="restore-toggle"
      @click="toggleCollapse"
      aria-label="展开导航"
    >
      <NIcon class="chevron-icon restore">
        <ChevronDown />
      </NIcon>
    </button>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { RouterLink } from 'vue-router';
import { NIcon } from 'naive-ui';
import { 
  ChevronDown, 
  PersonOutline, 
  StorefrontOutline, 
  SettingsOutline 
} from '@vicons/ionicons5';

const isOpen = ref(false);
const isCollapsed = ref(localStorage.getItem('header_collapsed') === 'true');

const closeIfMobile = () => {
  if (window.innerWidth <= 768) {
    isOpen.value = false;
  }
};

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value;
  localStorage.setItem('header_collapsed', isCollapsed.value);
  if (isCollapsed.value) {
    isOpen.value = false;
  }
};
</script>

<style scoped>
/* 通用样式保持不变，新增部分在下方 */
.app-header-container {
  position: relative;
  z-index: 1100;
  --app-header-height: 56px;
}

.app-header {
  position: sticky;
  top: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  padding-top: calc(6px + env(safe-area-inset-top, 0));
  border-bottom: 1px solid var(--border-color);
  background-color: var(--bg-color);
  min-height: var(--app-header-height, 64px);
  transition: all 0.25s ease;
  overflow: hidden;
}

.app-header-left {
  display: flex;
  align-items: center;
  min-height: 36px;
  min-width: 0;
}

.app-logo {
  font-size: 16px;
  font-weight: 600;
  color: var(--primary-text-color);
  margin-right: auto; /* 让 Logo 靠左，把右边的元素推过去 */
}

/* 恢复按钮样式 */
.restore-toggle {
  position: absolute;
  top: calc(6px + env(safe-area-inset-top, 0));
  right: 12px;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
  box-shadow: var(--shadow-sm);
  z-index: 10;
}

/* 收起按钮样式 */
.collapse-toggle {
  margin-left: 12px;
  width: 36px;
  height: 36px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
  background: var(--bg-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
}

.chevron-icon {
  font-size: 18px;
  transform: rotate(90deg);
  transition: transform 0.2s ease;
  color: var(--primary-text-color);
}

.chevron-icon.closed,
.chevron-icon.restore {
  transform: rotate(-90deg);
}

/* 汉堡菜单按钮 */
.menu-toggle {
  display: none; /* 桌面端隐藏 */
  width: 36px;
  height: 36px;
  margin-left: 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  cursor: pointer;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.menu-icon,
.menu-icon::before,
.menu-icon::after {
  display: block;
  position: relative;
  width: 16px;
  height: 2px;
  background: var(--primary-text-color);
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.menu-icon::before,
.menu-icon::after {
  content: '';
  position: absolute;
  left: 0;
}

.menu-icon::before { top: -5px; }
.menu-icon::after { top: 5px; }

.menu-icon.open { transform: rotate(45deg); }
.menu-icon.open::before { transform: rotate(-90deg) translate(-5px, 0); }
.menu-icon.open::after { opacity: 0; }

/* 导航链接 */
.app-header-nav {
  display: flex;
  gap: 8px;
}

.nav-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-radius: var(--radius-md);
  font-size: 13px;
  color: var(--secondary-text-color);
  text-decoration: none;
  transition: background-color 0.15s ease, color 0.15s ease;
}

.nav-link:hover {
  background-color: var(--bg-secondary);
  color: var(--primary-text-color);
}

.nav-link--active {
  background-color: var(--accent-color);
  color: var(--bg-color); /* 假设 accent-color 是深色，文字变亮色 */
}

.nav-icon-desktop {
  font-size: 16px;
}

/* 收起状态 */
.app-header.collapsed {
  min-height: 0;
  max-height: 0;
  padding-left: 0;
  padding-right: 0;
  padding-top: 0;
  padding-bottom: 0;
  border-bottom: 0;
  opacity: 0;
  transform: translateY(-6px);
  pointer-events: none;
}

/* ---------------------------------------------------------
   移动端专属样式 
   --------------------------------------------------------- */
.mobile-shortcuts {
  display: none; /* 桌面端隐藏 */
}

@media (max-width: 768px) {
  .app-header-container {
    --app-header-height: 52px;
  }

  .app-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
    padding: 8px 12px;
    padding-top: calc(4px + env(safe-area-inset-top, 0));
    min-height: var(--app-header-height, 56px);
  }

  .app-header-left {
    width: 100%;
    display: flex;
    align-items: center;
    min-height: 36px;
  }

  /* 移动端快捷按钮组 */
  .mobile-shortcuts {
    display: flex;
    gap: 8px;
    margin-left: auto;
    margin-right: 8px; /* 与汉堡菜单保持距离 */
  }

  .shortcut-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background-color: var(--bg-secondary);
    color: var(--secondary-text-color);
    font-size: 18px; /* 图标大小 */
    text-decoration: none;
    border: 1px solid transparent;
    transition: all 0.2s;
  }
  
  /* 如果你使用汉字，可以在这里调整字体 */
  /* .shortcut-btn span { font-size: 14px; font-weight: bold; } */

  .shortcut-btn--active {
    background-color: var(--accent-color);
    color: var(--bg-color);
    border-color: var(--accent-color);
  }

  .menu-toggle {
    display: inline-flex;
  }

  /* 完整的导航菜单 (展开后) */
  .app-header-nav {
    width: 100%;
    flex-direction: column;
    gap: 4px;
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1), margin-top 0.3s ease;
    margin-top: 0;
  }

  .app-header-nav.open {
    max-height: 200px; /* 足够容纳菜单的高度 */
    margin-top: 8px;
    border-top: 1px solid var(--border-color); /* 可选：增加分割线 */
    padding-top: 8px;
    padding-bottom: calc(4px + env(safe-area-inset-bottom, 0));
  }

  .nav-link {
    width: 100%;
    padding: 9px 12px; /* 增加移动端点击区域 */
    justify-content: flex-start;
  }

  /* 移动端调整收起整个 Header 的按钮位置 */
  .collapse-toggle {
    display: none; /* 移动端通常不需要这个收起Header的按钮，因为它会和汉堡菜单逻辑冲突。
                      如果必须保留，建议放在 nav 底部或者绝对定位 */
  }
}
</style>
