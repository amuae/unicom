import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'

import App from './App.vue'
import router from './router'

const app = createApp(App)

// 注册 Element Plus 图标
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

app.use(createPinia())
app.use(router)
app.use(ElementPlus)

app.mount('#app')

// 版本检查：定期轮询 /api/version，版本变化时自动刷新
let currentVersion: string | null = null

async function checkVersion() {
  try {
    const res = await fetch('/version')
    const data = await res.json()
    if (currentVersion === null) {
      currentVersion = data.version
    } else if (data.version !== currentVersion) {
      console.log(`版本更新: ${currentVersion} -> ${data.version}，刷新页面...`)
      window.location.reload()
    }
  } catch {
    // 忽略网络错误
  }
}

// 每 60 秒检查一次版本
setInterval(checkVersion, 60000)
checkVersion()
