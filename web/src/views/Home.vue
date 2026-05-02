<template>
  <div class="home">
    <!-- 导航栏 -->
    <nav class="navbar">
      <div class="nav-content">
        <div class="nav-brand">
          <span class="brand-icon">📱</span>
          <span class="brand-text">UNICOM</span>
        </div>
        <div class="nav-links">
          <a @click="goToLogin" class="nav-link">登录</a>
          <a @click="goToQuery" class="nav-btn">开始查询</a>
        </div>
      </div>
    </nav>

    <!-- 主内容 -->
    <main class="main">
      <!-- Hero 区域 -->
      <section class="hero">
        <div class="hero-content">
          <div class="hero-badge">FLOW MONITOR</div>
          <h1 class="hero-title">
            联通流量<br />
            <span class="highlight">实时查询系统</span>
          </h1>
          <p class="hero-desc">
            自动定时查询 · 消息实时提醒 · 多用户管理
          </p>
          <div class="hero-actions">
            <button @click="goToLogin" class="primary-btn">
              登录 / 注册
            </button>
            <button @click="goToQuery" class="secondary-btn">
              直接查询 →
            </button>
          </div>
        </div>
      </section>

      <!-- 功能特性 -->
      <section class="features">
        <div class="section-header">
          <div class="section-badge">FEATURES</div>
          <h2>核心功能</h2>
          <p>一站式流量管理解决方案</p>
        </div>
        <div class="feature-grid">
          <div v-for="f in features" :key="f.icon" class="feature-card">
            <div class="feature-icon">{{ f.icon }}</div>
            <h3>{{ f.title }}</h3>
            <p>{{ f.desc }}</p>
          </div>
        </div>
      </section>

      <!-- 使用流程 -->
      <section class="steps">
        <div class="section-header">
          <div class="section-badge">HOW TO USE</div>
          <h2>如何使用</h2>
          <p>三步开始监控您的流量</p>
        </div>
        <div class="steps-grid">
          <div v-for="(s, i) in steps" :key="i" class="step-card">
            <div class="step-num">{{ String(i + 1).padStart(2, '0') }}</div>
            <h3>{{ s.title }}</h3>
            <p>{{ s.desc }}</p>
          </div>
        </div>
      </section>

      <!-- 状态 -->
      <section class="status-section">
        <div class="status-card">
          <div class="status-header">
            <span class="status-dot"></span>
            <span>系统状态</span>
          </div>
          <div class="status-grid">
            <div class="status-item">
              <span class="status-label">服务</span>
              <span class="status-value online">运行中</span>
            </div>
            <div class="status-item">
              <span class="status-label">版本</span>
              <span class="status-value">v1.0.0</span>
            </div>
            <div class="status-item">
              <span class="status-label">数据库</span>
              <span class="status-value online">正常</span>
            </div>
          </div>
        </div>
      </section>

      <!-- 底部 -->
      <footer class="footer">
        <div class="footer-content">
          <div class="footer-brand">
            <span>📱</span> UNICOM - 联通流量查询系统
          </div>
          <div class="footer-copy">© 2026 All rights reserved</div>
        </div>
      </footer>
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const features = [
  { icon: '📊', title: '实时查询', desc: '随时查看流量使用详情，支持多套餐分类展示' },
  { icon: '⏰', title: '定时监控', desc: '自定义 Cron 表达式，自动定时查询流量' },
  { icon: '🔔', title: '消息提醒', desc: '流量达到阈值自动推送 Telegram 通知' },
  { icon: '👥', title: '多用户管理', desc: '支持多个联通账号统一管理' },
  { icon: '📈', title: '数据统计', desc: '流量使用趋势分析，历史记录查询' },
  { icon: '🔒', title: '安全可靠', desc: 'Token 鉴权，数据本地存储' },
]

const steps = [
  { title: '注册账号', desc: '使用手机号注册，获取专属查询 Token' },
  { title: '配置通知', desc: '设置 Telegram Bot 和提醒阈值' },
  { title: '自动监控', desc: '系统定时查询，流量异常自动通知' },
]

const goToLogin = () => router.push('/login')
const goToQuery = () => router.push('/login')
</script>

<style scoped>
/* 字体 */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap');

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

.home {
  min-height: 100vh;
  background: #f5f0e8;
  color: #2c2c2c;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}

/* 导航栏 */
.navbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 56px;
  background: #f5f0e8;
  border-bottom: 2px dashed rgba(44, 44, 44, 0.15);
  z-index: 100;
}

.nav-content {
  max-width: 1100px;
  margin: 0 auto;
  padding: 0 24px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.nav-brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-icon {
  font-size: 20px;
}

.brand-text {
  font-size: 16px;
  font-weight: 800;
  color: #2c2c2c;
  letter-spacing: 1px;
  font-family: 'JetBrains Mono', monospace;
}

.nav-links {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-link {
  font-size: 13px;
  color: rgba(44, 44, 44, 0.6);
  cursor: pointer;
  transition: color 0.2s;
  font-weight: 500;
}

.nav-link:hover {
  color: #2c2c2c;
}

.nav-btn {
  background: #2c2c2c;
  color: #f5f0e8;
  padding: 6px 16px;
  border-radius: 2px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px dashed #2c2c2c;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.2);
}

.nav-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 3px 3px rgba(44, 44, 44, 0.3);
}

/* 主内容 */
.main {
  padding-top: 56px;
}

/* Hero */
.hero {
  padding: 80px 24px 60px;
  text-align: center;
}

.hero-content {
  max-width: 650px;
  margin: 0 auto;
}

.hero-badge {
  display: inline-block;
  background: rgba(180, 71, 47, 0.1);
  color: #b4472f;
  padding: 4px 12px;
  border-radius: 2px;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 2px;
  font-family: 'JetBrains Mono', monospace;
  border: 1px dashed rgba(180, 71, 47, 0.3);
  margin-bottom: 24px;
}

.hero-title {
  font-size: 48px;
  font-weight: 800;
  line-height: 1.15;
  margin: 0 0 20px;
  color: #2c2c2c;
}

.highlight {
  color: #b4472f;
}

.hero-desc {
  font-size: 16px;
  color: rgba(44, 44, 44, 0.6);
  margin: 0 0 36px;
  letter-spacing: 3px;
}

.hero-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.primary-btn {
  height: 44px;
  padding: 0 28px;
  font-size: 14px;
  font-weight: 700;
  border-radius: 2px;
  background: #2c2c2c;
  border: 2px dashed #2c2c2c;
  color: #f5f0e8;
  cursor: pointer;
  box-shadow: 3px 3px rgba(44, 44, 44, 0.25);
  transition: all 0.15s;
  font-family: inherit;
}

.primary-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px rgba(44, 44, 44, 0.35);
}

.secondary-btn {
  height: 44px;
  padding: 0 28px;
  font-size: 14px;
  font-weight: 600;
  border-radius: 2px;
  background: transparent;
  border: 2px dashed rgba(44, 44, 44, 0.25);
  color: #2c2c2c;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.secondary-btn:hover {
  border-color: #2c2c2c;
  background: rgba(44, 44, 44, 0.05);
}

/* 通用区块 */
.section-header {
  text-align: center;
  margin-bottom: 40px;
}

.section-badge {
  display: inline-block;
  background: rgba(44, 44, 44, 0.06);
  color: rgba(44, 44, 44, 0.5);
  padding: 3px 10px;
  border-radius: 2px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 2px;
  font-family: 'JetBrains Mono', monospace;
  border: 1px dashed rgba(44, 44, 44, 0.12);
  margin-bottom: 12px;
}

.section-header h2 {
  font-size: 28px;
  font-weight: 800;
  margin: 0 0 8px;
  color: #2c2c2c;
}

.section-header p {
  font-size: 14px;
  color: rgba(44, 44, 44, 0.5);
  margin: 0;
}

/* 功能特性 */
.features {
  max-width: 1100px;
  margin: 0 auto;
  padding: 60px 24px;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.feature-card {
  background: #faf5ed;
  border: 2px dashed rgba(44, 44, 44, 0.15);
  border-radius: 2px;
  padding: 24px 20px;
  transition: all 0.2s;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.06);
}

.feature-card:hover {
  transform: translate(-2px, -2px);
  box-shadow: 4px 4px rgba(44, 44, 44, 0.12);
  border-color: rgba(44, 44, 44, 0.3);
}

.feature-icon {
  font-size: 28px;
  margin-bottom: 12px;
}

.feature-card h3 {
  font-size: 15px;
  font-weight: 700;
  margin: 0 0 6px;
  color: #2c2c2c;
}

.feature-card p {
  font-size: 13px;
  color: rgba(44, 44, 44, 0.6);
  margin: 0;
  line-height: 1.5;
}

/* 使用流程 */
.steps {
  max-width: 1100px;
  margin: 0 auto;
  padding: 60px 24px;
}

.steps-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 24px;
}

.step-card {
  text-align: center;
  padding: 24px;
  background: #faf5ed;
  border: 2px dashed rgba(44, 44, 44, 0.12);
  border-radius: 2px;
}

.step-num {
  font-family: 'JetBrains Mono', monospace;
  font-size: 32px;
  font-weight: 800;
  color: #b4472f;
  margin-bottom: 12px;
}

.step-card h3 {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 8px;
  color: #2c2c2c;
}

.step-card p {
  font-size: 13px;
  color: rgba(44, 44, 44, 0.6);
  margin: 0;
  line-height: 1.5;
}

/* 状态 */
.status-section {
  max-width: 1100px;
  margin: 0 auto;
  padding: 0 24px 60px;
}

.status-card {
  background: #faf5ed;
  border: 2px dashed rgba(44, 44, 44, 0.15);
  border-radius: 2px;
  padding: 20px;
  box-shadow: 2px 2px rgba(44, 44, 44, 0.06);
}

.status-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  color: #2c2c2c;
  margin-bottom: 16px;
  font-family: 'JetBrains Mono', monospace;
  letter-spacing: 1px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #22c55e;
  box-shadow: 0 0 0 3px rgba(34, 197, 94, 0.2);
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.status-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  background: rgba(44, 44, 44, 0.03);
  border: 1px dashed rgba(44, 44, 44, 0.1);
  border-radius: 2px;
}

.status-label {
  font-size: 12px;
  color: rgba(44, 44, 44, 0.5);
  font-weight: 500;
}

.status-value {
  font-size: 12px;
  font-weight: 700;
  color: #2c2c2c;
  font-family: 'JetBrains Mono', monospace;
}

.status-value.online {
  color: #22c55e;
}

/* 底部 */
.footer {
  margin-top: 0;
  padding: 20px 24px;
  border-top: 2px dashed rgba(44, 44, 44, 0.12);
}

.footer-content {
  max-width: 1100px;
  margin: 0 auto;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.footer-brand {
  font-size: 13px;
  font-weight: 600;
  color: #2c2c2c;
  display: flex;
  align-items: center;
  gap: 6px;
}

.footer-copy {
  font-size: 12px;
  color: rgba(44, 44, 44, 0.4);
  font-family: 'JetBrains Mono', monospace;
}

/* 响应式 */
@media (max-width: 768px) {
  .hero-title {
    font-size: 32px;
  }

  .hero-desc {
    font-size: 14px;
  }

  .hero-actions {
    flex-direction: column;
    align-items: center;
  }

  .primary-btn,
  .secondary-btn {
    width: 100%;
    max-width: 260px;
  }

  .feature-grid,
  .steps-grid,
  .status-grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .footer-content {
    flex-direction: column;
    gap: 8px;
    text-align: center;
  }
}
</style>
