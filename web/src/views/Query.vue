<template>
  <div class="query-page">
    <!-- Loading Overlay -->
    <div class="loading-overlay" :class="{ show: loading }">
      <div class="spinner"></div>
      <div class="loading-text">{{ loadingText }}</div>
    </div>

    <div class="container">
      <!-- Error Card -->
      <div class="error-card" v-if="error">
        <div class="error-message">{{ error }}</div>
        <button class="btn-retry" @click="queryFlow">重试</button>
      </div>

      <!-- Header -->
      <div class="header" v-if="data">
        <div class="header-top">
          <div>
            <div class="package-name">{{ data.main_package || '中国联通' }}</div>
            <div class="mobile-number">{{ maskedMobile }}</div>
          </div>
          <div class="header-actions">
            <button class="header-icon-btn" @click="queryFlow" :disabled="refreshing" title="刷新">🔄</button>
          </div>
        </div>
        <div class="header-stats">
          <div class="stat-item">
            <div class="stat-label">💰 话费余额</div>
            <div class="stat-value">¥{{ data.balance?.balance || '0.00' }}</div>
          </div>
          <div class="stat-item">
            <div class="stat-label">📊 当月出账</div>
            <div class="stat-value">¥{{ data.balance?.real_fee || '0.00' }}</div>
          </div>
        </div>
      </div>

      <!-- Summary Card (Buckets) -->
      <div class="summary-card" v-if="data && bucketList.length > 0">
        <div class="bucket-scroll-container">
          <div class="bucket-scroll-wrapper">
            <div v-for="b in bucketList" :key="b.key" class="bucket-mini-card" :class="b.type">
              <div class="bucket-mini-name">{{ b.name }}</div>
              <div class="bucket-mini-used">本次: {{ formatFlow(b.uused) }}</div>
              <div class="bucket-mini-detail">
                <div>📆 今日: {{ formatFlow(b.today) }}</div>
                <div>💾 已用: {{ formatFlow(b.used) }}</div>
                <div>📦 剩余: {{ b.unlimited ? '无限' : formatFlow(b.remain) }}</div>
              </div>
            </div>
          </div>
        </div>
        <div class="summary-footer">
          <div><span>时长:</span> <span id="timeInterval">{{ data.timeInterval || '首次查询' }}</span></div>
          <div><span>时间:</span> <span id="updateTime">{{ data.timestamp || '' }}</span></div>
        </div>
      </div>

      <!-- Packages -->
      <div id="packagesContainer" v-if="data">
        <!-- Normal packages -->
        <div v-for="(pkg, idx) in normalPackages" :key="'pkg-' + idx" class="package-card">
          <div class="package-header">
            <div class="package-name">{{ pkg.name || '未知套餐' }}</div>
            <span v-if="pkg.is_public_free" class="package-badge">免费</span>
          </div>
          <div class="package-info">
            <span class="package-used">{{ formatFlow(pkg.use || 0) }} / {{ isPkgUnlimited(pkg) ? '∞' : formatFlow(pkg.total || 0) }}</span>
            <span class="package-percent">{{ isPkgUnlimited(pkg) ? '不限量' : pkgPercent(pkg) + '%' }}</span>
          </div>
          <div v-if="!isPkgUnlimited(pkg)" class="package-bar">
            <div class="package-bar-fill" :style="{ width: Math.min(pkgPercent(pkg), 100) + '%' }"></div>
          </div>
          <div class="package-detail">
            <span>剩余 {{ isPkgUnlimited(pkg) ? '∞' : formatFlow(pkg.remain || 0) }}</span>
            <span v-if="pkg.end_date && pkg.end_date !== '长期有效'" style="color: #ff9800">⏰ {{ pkg.end_date }}</span>
            <span v-else-if="pkg.end_date === '长期有效'" style="color: #4caf50">✓ 长期有效</span>
          </div>
          <!-- Vice cards -->
          <div v-if="pkg.viceCardlist && pkg.viceCardlist.length > 0" class="vice-card">
            <div class="vice-title" @click="toggleVice(idx)">
              🔗 主副卡使用详情
              <span class="vice-toggle" :class="{ collapsed: !viceExpanded[idx] }">▼</span>
            </div>
            <div v-show="viceExpanded[idx]" class="vice-content">
              <div v-for="(vice, vi) in pkg.viceCardlist" :key="vi" class="vice-item">
                <div>
                  <span class="vice-number">{{ vice.usernumber }}</span>
                  <span v-if="vice.currentLoginFlag === '1'" class="vice-current">（当前登录）</span>
                  <span v-if="vice.viceCardflag === '1'" style="color:#999;font-size:11px">（主卡）</span>
                  <span v-else style="color:#999;font-size:11px">（副卡）</span>
                </div>
                <span class="vice-usage">{{ formatFlow(parseFloat(vice.use || 0)) }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Public free packages -->
        <div v-if="publicFreePackages.length > 0" class="package-card">
          <div class="package-header">
            <div class="package-name">公免流量</div>
            <span class="package-badge">免费</span>
          </div>
          <div class="vice-card">
            <div class="vice-title">🎁 公免流量详情</div>
            <div v-for="(pkg, pi) in publicFreePackages" :key="'pf-' + pi" class="vice-item">
              <div style="flex:1">
                <div style="font-weight:500;color:#333;margin-bottom:4px">{{ pkg.name }}</div>
                <div style="font-size:11px;color:#999">已用 {{ formatFlow(pkg.use || 0) }} / {{ isPkgUnlimited(pkg) ? '∞' : formatFlow(pkg.total || 0) }}</div>
              </div>
              <span class="vice-usage">{{ isPkgUnlimited(pkg) ? '不限量' : pkgPercent(pkg) + '%' }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="footer" v-if="data">
        <button class="btn-reset" @click="resetStats">🔄 重置统计周期</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, reactive } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const token = route.params.token as string

const loading = ref(false)
const loadingText = ref('正在查询流量...')
const refreshing = ref(false)
const error = ref('')
const data = ref<any>(null)
const viceExpanded = ref<Record<number, boolean>>({})

const maskedMobile = computed(() => {
  const m = data.value?.mobile || ''
  return m.length >= 7 ? m.slice(0, 3) + '****' + m.slice(-4) : m
})

const bucketOrder = [
  { key: '所有通用', name: '📱 所有通用', type: 'common' },
  { key: '所有免流', name: '🎯 所有免流', type: 'targeted' },
  { key: '所有流量', name: '📊 所有流量', type: 'common' },
  { key: 'common_limited', name: '通用有限', type: 'common' },
  { key: 'common_unlimited', name: '通用不限', type: 'common' },
  { key: 'regional_limited', name: '区域有限', type: 'regional' },
  { key: 'regional_unlimited', name: '区域不限', type: 'regional' },
  { key: 'targeted_limited', name: '免流有限', type: 'targeted' },
  { key: 'targeted_unlimited', name: '免流不限', type: 'targeted' },
]

const bucketList = computed(() => {
  if (!data.value?.buckets) return []
  const buckets = data.value.buckets
  const diff = data.value.diff || {}
  return bucketOrder
    .filter(item => {
      const b = buckets[item.key]
      return b && (b.total > 0 || b.used > 0 || b.remain !== 0)
    })
    .map(item => {
      const b = buckets[item.key]
      const d = diff[item.key] || { uused: 0, today: 0 }
      return { ...item, total: b.total, used: b.used, remain: b.remain, uused: d.uused || 0, today: d.today || 0, unlimited: b.total >= 999999 || item.key.includes('不限') }
    })
})

const normalPackages = computed(() => (data.value?.packages || []).filter((p: any) => !p.is_public_free))
const publicFreePackages = computed(() => (data.value?.packages || []).filter((p: any) => p.is_public_free))

function formatFlow(mb: number): string {
  if (!mb || mb === 0) return '0'
  if (Math.abs(mb) >= 1048576) return (mb / 1048576).toFixed(2) + 'TB'
  if (Math.abs(mb) >= 1024) return (mb / 1024).toFixed(2) + 'GB'
  return mb.toFixed(2) + 'MB'
}

function isPkgUnlimited(pkg: any): boolean {
  return (pkg.total || 0) >= 999999 || (pkg.total === 0 && (pkg.remain || 0) < 0)
}

function pkgPercent(pkg: any): number {
  const t = pkg.total || 0
  if (t <= 0 || t >= 999999) return 0
  return parseFloat(((pkg.use || 0) / t * 100).toFixed(1))
}

function toggleVice(idx: number) {
  viceExpanded.value[idx] = !viceExpanded.value[idx]
}

async function queryFlow() {
  loading.value = true
  loadingText.value = '正在查询流量...'
  error.value = ''
  refreshing.value = true
  try {
    const res = await fetch(`/query/flow/${token}`)
    const result = await res.json()
    if (result.success) {
      data.value = result.data
    } else {
      error.value = result.error || '查询失败'
    }
  } catch (e: any) {
    error.value = '网络错误: ' + e.message
  } finally {
    loading.value = false
    setTimeout(() => { refreshing.value = false }, 2000)
  }
}

async function resetStats() {
  if (!confirm('确定要重置统计周期吗？\n重置后将以当前查询结果作为新的基准点。')) return
  loading.value = true
  loadingText.value = '正在重置...'
  try {
    const res = await fetch(`/user/${token}/reset`, { method: 'POST' })
    const result = await res.json()
    if (result.success) {
      alert('✅ 统计周期已重置')
      queryFlow()
    } else {
      alert('❌ 重置失败: ' + (result.error || ''))
    }
  } catch (e: any) {
    alert('❌ 网络错误: ' + e.message)
  } finally {
    loading.value = false
  }
}

onMounted(() => { queryFlow() })
</script>

<style scoped>
.query-page {
  background: #f5f0e8;
  min-height: 100vh;
  padding: 15px;
  color: #2c2c2c;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "PingFang SC", sans-serif;
}
.container { max-width: 600px; margin: 0 auto; }

/* Loading */
.loading-overlay { position: fixed; inset: 0; background: #f5f0e8; display: none; flex-direction: column; justify-content: center; align-items: center; z-index: 9999; }
.loading-overlay.show { display: flex; }
.spinner { width: 50px; height: 50px; border: 4px dashed rgba(180,71,47,0.3); border-top-color: #b4472f; border-radius: 2px; animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.loading-text { color: #2c2c2c; margin-top: 20px; font-size: 16px; font-weight: 500; font-family: 'JetBrains Mono', monospace; }

/* Error */
.error-card { background: #faf5ed; border-radius: 2px; padding: 18px; margin-bottom: 15px; box-shadow: 2px 2px 0 #d4cfc7; border: 2px dashed #f56c6c; }
.error-message { color: #f56c6c; font-size: 14px; margin-bottom: 10px; }
.btn-retry { background: #b4472f; color: #faf5ed; border: 2px solid #2c2c2c; padding: 8px 20px; border-radius: 2px; cursor: pointer; font-size: 14px; font-family: 'JetBrains Mono', monospace; box-shadow: 2px 2px 0 #2c2c2c; }
.btn-retry:hover { background: #9a3c27; }

/* Header */
.header { background: #faf5ed; border-radius: 2px; padding: 18px; margin-bottom: 15px; box-shadow: 2px 2px 0 #d4cfc7; border: 2px dashed #d4cfc7; }
.header-top { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 12px; }
.package-name { font-size: 16px; color: #2c2c2c; font-weight: 600; }
.mobile-number { font-size: 13px; color: #888; margin-top: 2px; font-family: 'JetBrains Mono', monospace; }
.header-actions { display: flex; gap: 8px; }
.header-icon-btn { width: 36px; height: 36px; border: 2px solid #2c2c2c; border-radius: 2px; background: #faf5ed; font-size: 16px; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; justify-content: center; box-shadow: 2px 2px 0 #2c2c2c; }
.header-icon-btn:hover { background: #b4472f; color: #faf5ed; }
.header-icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.header-stats { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.stat-item { background: #f5f0e8; border-radius: 2px; padding: 10px 12px; border: 2px dashed #d4cfc7; }
.stat-label { font-size: 12px; color: #888; margin-bottom: 4px; }
.stat-value { font-size: 18px; font-weight: 700; color: #2c2c2c; font-family: 'JetBrains Mono', monospace; }

/* Summary */
.summary-card { background: #faf5ed; border-radius: 2px; padding: 18px; margin-bottom: 15px; box-shadow: 2px 2px 0 #d4cfc7; border: 2px dashed #d4cfc7; }
.bucket-scroll-container { overflow-x: auto; overflow-y: hidden; -webkit-overflow-scrolling: touch; scrollbar-width: none; margin: 0 -18px; padding: 0 18px 10px; cursor: grab; user-select: none; touch-action: pan-x; min-width: 0; }
.bucket-scroll-container::-webkit-scrollbar { display: none; }
.bucket-scroll-wrapper { display: flex; gap: 12px; width: max-content; }
.bucket-mini-card { background: #f5f0e8; border-radius: 2px; padding: 12px 14px; min-width: 160px; flex-shrink: 0; border-left: 3px solid #b4472f; border: 2px dashed #d4cfc7; border-left: 3px solid #b4472f; box-shadow: 2px 2px 0 #d4cfc7; }
.bucket-mini-card.targeted { background: #faf5ed; border-left-color: #d4775f; }
.bucket-mini-card.regional { background: #f0f5ed; border-left-color: #5a8a5a; }
.bucket-mini-name { font-size: 12px; color: #666; margin-bottom: 8px; font-weight: 500; }
.bucket-mini-used { font-size: 16px; font-weight: bold; color: #2c2c2c; margin-bottom: 6px; font-family: 'JetBrains Mono', monospace; }
.bucket-mini-detail { font-size: 11px; color: #888; line-height: 1.4; }
.bucket-mini-detail div { margin-bottom: 2px; }
.summary-footer { padding: 10px 0 0; border-top: 2px dashed #d4cfc7; font-size: 12px; color: #666; display: flex; justify-content: space-between; }
.summary-footer #timeInterval, .summary-footer #updateTime { color: #b4472f; font-weight: 600; margin-left: 4px; font-family: 'JetBrains Mono', monospace; }

/* Packages */
.package-card { background: #faf5ed; border-radius: 2px; padding: 16px 18px; margin-bottom: 10px; box-shadow: 2px 2px 0 #d4cfc7; border: 2px dashed #d4cfc7; }
.package-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 8px; }
.package-name { font-size: 14px; font-weight: 600; color: #2c2c2c; flex: 1; line-height: 1.4; }
.package-badge { background: #b4472f; color: #faf5ed; padding: 2px 8px; border-radius: 2px; font-size: 11px; margin-left: 8px; flex-shrink: 0; border: 1px solid #2c2c2c; font-family: 'JetBrains Mono', monospace; }
.package-info { display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #666; margin-bottom: 8px; }
.package-used, .package-percent { font-weight: 600; color: #b4472f; font-family: 'JetBrains Mono', monospace; }
.package-bar { height: 6px; background: #e8e3db; border-radius: 2px; overflow: hidden; margin-bottom: 8px; border: 1px dashed #d4cfc7; }
.package-bar-fill { height: 100%; background: #b4472f; border-radius: 2px; transition: width 0.5s ease; }
.package-detail { display: flex; justify-content: space-between; font-size: 11px; color: #888; }

/* Vice cards */
.vice-card { background: #f5f0e8; border-radius: 2px; padding: 12px; margin-top: 10px; font-size: 12px; color: #666; border: 2px dashed #d4cfc7; }
.vice-title { font-weight: 600; cursor: pointer; display: flex; justify-content: space-between; align-items: center; }
.vice-toggle { transition: transform 0.3s; font-size: 10px; }
.vice-toggle.collapsed { transform: rotate(-90deg); }
.vice-content { margin-top: 8px; }
.vice-item { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; border-bottom: 1px dashed #d4cfc7; }
.vice-item:last-child { border-bottom: none; }
.vice-number { font-weight: 500; color: #2c2c2c; font-family: 'JetBrains Mono', monospace; }
.vice-current { color: #b4472f; font-size: 11px; font-weight: 600; }
.vice-usage { font-weight: 600; color: #b4472f; font-family: 'JetBrains Mono', monospace; }

/* Footer */
.footer { text-align: center; padding: 15px 0; }
.btn-reset { background: #faf5ed; color: #2c2c2c; border: 2px solid #2c2c2c; padding: 10px 24px; border-radius: 2px; cursor: pointer; font-size: 14px; box-shadow: 2px 2px 0 #2c2c2c; font-family: 'JetBrains Mono', monospace; }
.btn-reset:hover { background: #b4472f; color: #faf5ed; }
</style>
