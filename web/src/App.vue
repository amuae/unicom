<template>
  <el-config-provider :locale="zhCn">
    <router-view />
  </el-config-provider>
</template>

<script setup lang="ts">
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
</script>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap');

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background-color: #f5f0e8;
  color: #2c2c2c;
  -webkit-font-smoothing: antialiased;
}

/* Element Plus 主题覆盖 */
:root {
  --el-color-primary: #b4472f;
  --el-color-primary-light-3: #c96b56;
  --el-color-primary-light-5: #d99580;
  --el-color-primary-light-7: #e8bbaa;
  --el-color-primary-light-8: #f0d0c3;
  --el-color-primary-light-9: #f8e8e0;
  --el-color-primary-dark-2: #903925;
  --el-border-radius-base: 2px;
  --el-font-family: 'Inter', sans-serif;
}

/* Element Plus 组件样式覆盖 */
.el-button {
  border-radius: 2px !important;
  font-family: 'Inter', sans-serif;
  font-weight: 600;
}

.el-button--primary {
  background-color: #2c2c2c !important;
  border-color: #2c2c2c !important;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.2);
}

.el-button--primary:hover {
  transform: translate(-1px, -1px);
  box-shadow: 3px 3px rgba(44, 44, 44, 0.3);
}

.el-input__wrapper {
  border-radius: 2px !important;
  border: 2px dashed rgba(44, 44, 44, 0.2) !important;
  box-shadow: none !important;
  background: #faf5ed !important;
}

.el-input__wrapper:hover {
  border-color: rgba(44, 44, 44, 0.4) !important;
}

.el-input__wrapper.is-focus {
  border-color: #2c2c2c !important;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.15) !important;
}

.el-card {
  border-radius: 2px !important;
  border: 2px dashed rgba(44, 44, 44, 0.15) !important;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.06) !important;
  background: #faf5ed !important;
}

.el-dialog {
  border-radius: 2px !important;
  border: 2px dashed rgba(44, 44, 44, 0.2) !important;
  box-shadow: 4px 4px rgba(44, 44, 44, 0.15) !important;
}

.el-table {
  border-radius: 2px !important;
  font-size: 13px;
}

.el-table th.el-table__cell {
  background: rgba(44, 44, 44, 0.04) !important;
  font-weight: 700;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.el-tag {
  border-radius: 2px !important;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
}

.el-tabs__item {
  font-weight: 600;
  font-size: 13px;
}

.el-tabs__active-bar {
  background-color: #2c2c2c;
}

.el-dialog__header {
  border-bottom: 2px dashed rgba(44, 44, 44, 0.1);
  padding: 16px 20px;
}

.el-dialog__footer {
  border-top: 2px dashed rgba(44, 44, 44, 0.1);
  padding: 12px 20px;
}

/* 滚动条 */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: rgba(44, 44, 44, 0.05);
}

::-webkit-scrollbar-thumb {
  background: rgba(44, 44, 44, 0.2);
  border-radius: 2px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(44, 44, 44, 0.35);
}
</style>
