<template>
  <Transition name="alert-fade">
    <div v-if="alertStore.isVisible" class="alert-overlay" @click.self="alertStore.hide">
      <div class="alert-box" :class="`alert-${alertStore.type}`">
        <header class="alert-header">
          <h3>{{ alertStore.title }}</h3>
          <button class="close-btn" @click="alertStore.hide">&times;</button>
        </header>
        <main class="alert-body">
          <p>{{ alertStore.message }}</p>
        </main>
        <footer class="alert-footer">
          <button class="ok-btn" @click="alertStore.hide">确认</button>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { useAlertStore } from '@/stores/alertStore';

const alertStore = useAlertStore();
</script>

<style scoped>
.alert-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: var(--overlay-color);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999999;
}

.alert-box {
  background-color: var(--alert-bg);
  border-radius: var(--radius-md);
  width: 90%;
  max-width: 400px;
  box-shadow: var(--shadow-xl);
  border-top: 4px solid var(--alert-info);
  color: var(--primary-text-color);
  transform: scale(1);
}

/* 根据类型改变边框颜色 */
.alert-box.alert-success { border-top-color: var(--alert-success); }
.alert-box.alert-warning { border-top-color: var(--alert-warning); }
.alert-box.alert-error { border-top-color: var(--alert-error); }

.alert-header {
  padding: 1rem 1.5rem;
  border-bottom: 1px solid var(--border-color-light);
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.alert-header h3 {
  margin: 0;
  font-size: var(--font-lg);
  font-weight: 500;
}
.close-btn {
  background: none;
  border: none;
  font-size: 2rem;
  color: var(--text-disabled);
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.alert-body {
  padding: 1.5rem;
  font-size: var(--font-md);
  line-height: 1.6;
}

.alert-footer {
  padding: 1rem 1.5rem;
  background-color: var(--bg-elevated);
  display: flex;
  justify-content: flex-end;
  border-radius: 0 0 var(--radius-md) var(--radius-md);
}

.ok-btn {
  background-color: var(--info-color);
  color: var(--text-white);
  border: none;
  padding: 0.75rem 2rem;
  border-radius: var(--radius-sm);
  font-weight: bold;
  cursor: pointer;
  transition: background-color 0.2s;
}
.ok-btn:hover {
  background-color: var(--info-color-hover);
}

/* 根据类型改变按钮颜色 */
.alert-success .ok-btn { background-color: var(--alert-success); }
.alert-success .ok-btn:hover { background-color: var(--success-color-hover); }
.alert-warning .ok-btn { background-color: var(--alert-warning); }
.alert-warning .ok-btn:hover { background-color: var(--warning-color-hover); }
.alert-error .ok-btn { background-color: var(--alert-error); }
.alert-error .ok-btn:hover { background-color: var(--error-color-hover); }

/* 过渡动画 */
.alert-fade-enter-active,
.alert-fade-leave-active {
  transition: opacity 0.3s ease;
}
.alert-fade-enter-active .alert-box,
.alert-fade-leave-active .alert-box {
  transition: transform 0.3s ease;
}

.alert-fade-enter-from,
.alert-fade-leave-to {
  opacity: 0;
}
.alert-fade-enter-from .alert-box,
.alert-fade-leave-to .alert-box {
  transform: scale(0.9);
}
</style>