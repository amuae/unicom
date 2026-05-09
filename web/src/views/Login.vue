<template>
  <div class="login-container">
    <!-- 背景装饰 -->
    <div class="bg-decoration">
      <div class="circle circle-1"></div>
      <div class="circle circle-2"></div>
      <div class="circle circle-3"></div>
    </div>

    <!-- 用户已登录状态 -->
    <div v-if="isUserLoggedIn" class="login-card glass-effect">
      <div class="login-header">
        <div class="logo-area">
          <span class="logo-icon">📱</span>
        </div>
        <h2>联通流量查询系统</h2>
        <p class="subtitle">用户信息</p>
      </div>
      <div class="user-info-section">
        <div class="info-item">
          <span class="info-icon">📱</span>
          <div class="info-content">
            <span class="info-label">手机号</span>
            <span class="info-value">{{ userInfo.mobile }}</span>
          </div>
        </div>
        <div class="info-item">
          <span class="info-icon">👤</span>
          <div class="info-content">
            <span class="info-label">昵称</span>
            <span class="info-value">{{ userInfo.nickname || '-' }}</span>
          </div>
        </div>
        <div class="info-item">
          <span class="info-icon">🔗</span>
          <div class="info-content">
            <span class="info-label">查询链接</span>
            <div class="token-cell">
              <el-tag size="small" type="info" class="token-tag">{{ userInfo.query_token }}</el-tag>
              <el-button size="small" text type="primary" @click="copyQueryLink" class="copy-btn">复制</el-button>
            </div>
          </div>
        </div>
      </div>
      <div class="action-buttons action-buttons-2x2">
        <button class="login-btn primary-btn" @click="goToQuery">
          <span class="btn-icon">📊</span> 查看流量
        </button>
        <button class="login-btn warning-btn" @click="openConfigModal">
          <span class="btn-icon">⚙️</span> 设置
        </button>
        <button class="login-btn secondary-btn" @click="userLogout">
          <span class="btn-icon">🚪</span> 退出登录
        </button>
        <button class="login-btn danger-btn" @click="confirmDeleteUser">
          <span class="btn-icon">🗑️</span> 删除账号
        </button>
      </div>
    </div>

    <!-- 未登录状态 -->
    <div v-else class="login-card glass-effect">
      <div class="login-header">
        <div class="logo-area">
          <span class="logo-icon">📱</span>
        </div>
        <h2>联通流量查询系统</h2>
        <p class="subtitle">登录以管理您的流量套餐</p>
      </div>

      <!-- Tab 切换：用户登录 / 用户注册 -->
      <el-tabs v-model="authTab" class="auth-tabs">
        <!-- 用户登录 -->
        <el-tab-pane label="用户登录" name="login">
          <el-form :model="userLoginForm" :rules="userLoginRules" ref="userLoginFormRef" class="login-form">
            <el-form-item prop="mobile">
              <el-input v-model="userLoginForm.mobile" placeholder="请输入手机号" prefix-icon="CellPhone" size="large" class="custom-input" />
            </el-form-item>
            <el-form-item prop="query_password">
              <el-input v-model="userLoginForm.query_password" type="password" placeholder="请输入查询密码" prefix-icon="Lock" size="large" show-password @keyup.enter="handleUserLogin" class="custom-input" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleUserLogin" :loading="userLoginLoading" size="large" class="login-btn primary-btn">
                <span class="btn-icon">🔐</span> 登录
              </el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <!-- 用户注册（仅当开启时显示） -->
        <el-tab-pane v-if="guestRegisterEnabled" label="用户注册" name="register">
          <el-form :model="registerForm" :rules="registerRules" ref="registerFormRef" class="login-form">
            <el-form-item prop="mobile">
              <el-input v-model="registerForm.mobile" placeholder="请输入手机号" size="large" class="custom-input" />
            </el-form-item>
            <el-form-item prop="query_password">
              <el-input v-model="registerForm.query_password" type="password" placeholder="设置查询密码" size="large" show-password class="custom-input" />
            </el-form-item>
            <el-form-item prop="nickname">
              <el-input v-model="registerForm.nickname" placeholder="请输入昵称" size="large" class="custom-input" />
            </el-form-item>
            <el-form-item prop="auth_type">
              <el-select v-model="registerForm.auth_type" placeholder="选择认证方式" size="large" class="custom-input" style="width:100%">
                <el-option label="Cookie" value="cookie" />
                <el-option label="Token Online" value="token_online" />
              </el-select>
            </el-form-item>
            <template v-if="registerForm.auth_type === 'cookie'">
              <el-form-item prop="cookie">
                <el-input v-model="registerForm.cookie" type="textarea" :rows="3" placeholder="粘贴 Cookie" size="large" class="custom-input" />
              </el-form-item>
            </template>
            <template v-if="registerForm.auth_type === 'token_online'">
              <el-form-item prop="appid">
                <el-input v-model="registerForm.appid" placeholder="App ID" size="large" class="custom-input" />
              </el-form-item>
              <el-form-item prop="token_online">
                <el-input v-model="registerForm.token_online" placeholder="Token Online" size="large" class="custom-input" />
              </el-form-item>
            </template>
            <el-form-item>
              <el-button type="success" @click="handleRegister" :loading="registerLoading" size="large" class="login-btn success-btn">
                <span class="btn-icon">✨</span> 注册
              </el-button>
            </el-form-item>
          </el-form>
        </el-tab-pane>
      </el-tabs>
    </div>

    <!-- 配置弹框 -->
    <div class="modal-overlay" :class="{ show: showModal }" @click.self="closeModal">
      <div class="modal">
        <div class="modal-header">
          <div class="modal-tabs">
            <button :class="{ active: activeTab === 'notify' }" @click="activeTab = 'notify'">通知配置</button>
            <button :class="{ active: activeTab === 'user' }" @click="activeTab = 'user'">用户配置</button>
          </div>
          <button class="modal-close" @click="closeModal">✕</button>
        </div>

        <!-- 通知配置 Tab -->
        <div v-show="activeTab === 'notify'" class="modal-body">
          <div class="form-group">
            <label class="form-label">
              <input type="checkbox" v-model="config.notify_enabled" :true-value="1" :false-value="0" /> 启用通知
            </label>
          </div>
          <div class="form-group">
            <label class="form-label">通知渠道</label>
            <select v-model="config.notify_type" class="form-input" @change="onNotifyTypeChange">
              <option value="">请选择</option>
              <option value="telegram">Telegram</option>
              <option value="wecom">企业微信</option>
              <option value="serverchan">Server酱</option>
              <option value="dingtalk">钉钉</option>
              <option value="pushplus">PushPlus</option>
            </select>
          </div>
          <div v-for="field in notifyFields" :key="field.name" class="form-group">
            <label class="form-label">{{ field.label }}</label>
            <input type="text" class="form-input" v-model="notifyParams[field.name]" :placeholder="field.placeholder" />
          </div>
          <div class="form-group">
            <label class="form-label">通知阈值 (MB)</label>
            <input type="number" class="form-input" v-model.number="config.notify_threshold" placeholder="5120" />
            <div class="form-hint">通用流量用量达到此值时发送通知，0=每次查询都通知</div>
          </div>
          <div class="form-group">
            <label class="form-label">查询间隔 (分钟)</label>
            <input type="number" class="form-input" v-model.number="config.query_interval" placeholder="30" />
          </div>
          <div class="form-group">
            <label class="form-label">通知标题</label>
            <input type="text" class="form-input" v-model="config.notify_title" placeholder="联通流量提醒" />
          </div>
          <div class="form-group">
            <label class="form-label">通知内容模板</label>
            <textarea class="form-input" v-model="config.notify_content" rows="4" placeholder="支持占位符: [套餐] [时间] [时长] [桶名.总量] [桶名.已用] [桶名.剩余] [桶名.用量] [桶名.今日用量]"></textarea>
          </div>
          <div class="modal-actions">
            <button class="btn-secondary" @click="testNotify">测试通知</button>
            <button class="btn-primary" @click="saveConfig">保存配置</button>
          </div>
        </div>

        <!-- 用户配置 Tab -->
        <div v-show="activeTab === 'user'" class="modal-body">
          <div class="form-group">
            <label class="form-label">昵称</label>
            <input type="text" class="form-input" v-model="config.nickname" />
          </div>
          <div class="form-group">
            <label class="form-label">查询密码</label>
            <input type="text" class="form-input" v-model="config.query_password" />
          </div>
          <div class="form-group">
            <label class="form-label">认证方式</label>
            <select v-model="config.auth_type" class="form-input">
              <option value="token_online">Token Online</option>
              <option value="cookie">Cookie</option>
            </select>
          </div>
          <div v-if="config.auth_type === 'token_online'" class="form-group">
            <label class="form-label">AppID</label>
            <input type="text" class="form-input" v-model="config.appid" />
          </div>
          <div v-if="config.auth_type === 'token_online'" class="form-group">
            <label class="form-label">Token Online</label>
            <textarea class="form-input" v-model="config.token_online" rows="3"></textarea>
          </div>
          <div class="form-group">
            <label class="form-label">当前 Cookie</label>
            <textarea class="form-input" :value="config.cookie" rows="3" readonly></textarea>
          </div>
          <div class="modal-actions">
            <button class="btn-primary" @click="saveConfig">保存配置</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import axios from 'axios'

const router = useRouter()
const loading = ref(false)
const userLoginLoading = ref(false)
const registerLoading = ref(false)
const guestRegisterEnabled = ref(false)

// ==================== 登录状态判断 ====================
const userLoggedIn = ref(!!localStorage.getItem('user_token'))

const isUserLoggedIn = computed(() => userLoggedIn.value || !!localStorage.getItem('user_token'))

const userInfo = reactive({
  mobile: localStorage.getItem('user_mobile') || '',
  nickname: localStorage.getItem('user_nickname') || '',
  query_token: localStorage.getItem('user_token') || '',
})

// ==================== Tab ====================
const authTab = ref('login')

// ==================== 用户登录 ====================
const userLoginFormRef = ref()
const userLoginForm = reactive({ mobile: '', query_password: '' })
const userLoginRules = {
  mobile: [
    { required: true, message: '请输入手机号', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '手机号格式不正确', trigger: 'blur' }
  ],
  query_password: [{ required: true, message: '请输入查询密码', trigger: 'blur' }],
}
const handleUserLogin = async () => {
  const formRef = userLoginFormRef.value
  if (!formRef) {
    ElMessage.error('表单未加载，请刷新页面')
    return
  }
  try {
    await formRef.validate()
  } catch {
    return
  }
  userLoginLoading.value = true
  try {
    const res = await axios.post('/auth/user-login', {
      mobile: userLoginForm.mobile,
      query_password: userLoginForm.query_password
    })
    if (res.data.success) {
      const token = res.data.data.token
      const user = res.data.data || {}
      localStorage.setItem('user_token', token)
      localStorage.setItem('user_mobile', user.mobile || userLoginForm.mobile)
      localStorage.setItem('user_nickname', user.nickname || '')
      userInfo.mobile = user.mobile || userLoginForm.mobile
      userInfo.nickname = user.nickname || ''
      userInfo.query_token = token
      userLoggedIn.value = true
      ElMessage.success('登录成功')
    }
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '登录失败') }
  finally { userLoginLoading.value = false }
}

const userLogout = () => {
  localStorage.removeItem('user_token')
  localStorage.removeItem('user_mobile')
  localStorage.removeItem('user_nickname')
  userLoggedIn.value = false
  userInfo.mobile = ''
  userInfo.nickname = ''
  userInfo.query_token = ''
  ElMessage.success('已退出')
}

const confirmDeleteUser = () => {
  ElMessageBox.confirm(
    '确定要删除自己的账号吗？此操作不可恢复，所有数据将被永久删除。',
    '删除账号',
    { confirmButtonText: '确定删除', cancelButtonText: '取消', type: 'warning' }
  ).then(() => deleteUser()).catch(() => {})
}

const deleteUser = async () => {
  try {
    const token = userInfo.query_token
    const res = await axios.delete(`/user/${token}`)
    if (res.data.success) {
      ElMessage.success('账号已删除')
      userLogout()
    } else {
      ElMessage.error(res.data.error || '删除失败')
    }
  } catch (e: any) {
    ElMessage.error(e.response?.data?.error || '删除失败')
  }
}

const copyQueryLink = () => {
  const url = `${window.location.origin}/query/${userInfo.query_token}`
  navigator.clipboard.writeText(url).then(() => {
    ElMessage.success('链接已复制')
  }).catch(() => {
    const input = document.createElement('input')
    input.value = url
    document.body.appendChild(input)
    input.select()
    document.execCommand('copy')
    document.body.removeChild(input)
    ElMessage.success('链接已复制')
  })
}

const goToQuery = () => {
  router.push(`/query/${userInfo.query_token}`)
}

// ==================== 注册 ====================
const registerFormRef = ref()
const registerForm = reactive({ mobile: '', query_password: '', nickname: '', auth_type: 'cookie', cookie: '', appid: '', token_online: '' })
const registerRules = {
  mobile: [
    { required: true, message: '请输入手机号', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '手机号格式不正确', trigger: 'blur' }
  ],
  query_password: [{ required: true, message: '请设置查询密码', trigger: 'blur' }],
  nickname: [{ required: true, message: '请输入昵称', trigger: 'blur' }],
  auth_type: [{ required: true, message: '请选择认证方式', trigger: 'change' }],
}
const handleRegister = async () => {
  if (!registerFormRef.value) { ElMessage.error('表单未加载，请刷新页面'); return }
  try { await registerFormRef.value.validate() } catch { return }
  if (registerForm.auth_type === 'cookie' && !registerForm.cookie) { ElMessage.error('请输入 Cookie'); return }
  if (registerForm.auth_type === 'token_online' && (!registerForm.appid || !registerForm.token_online)) { ElMessage.error('请输入 App ID 和 Token Online'); return }
  registerLoading.value = true
  try {
    const res = await axios.post('/auth/user-register', {
      mobile: registerForm.mobile,
      query_password: registerForm.query_password,
      nickname: registerForm.nickname,
      auth_type: registerForm.auth_type,
      cookie: registerForm.cookie,
      appid: registerForm.appid,
      token_online: registerForm.token_online,
    })
    if (res.data.message || res.data.success) {
      ElMessage.success('注册成功，请登录')
      userLoginForm.mobile = registerForm.mobile
      userLoginForm.query_password = ''
      registerForm.mobile = ''
      registerForm.query_password = ''
      registerForm.nickname = ''
      registerForm.auth_type = 'cookie'
      registerForm.cookie = ''
      registerForm.appid = ''
      registerForm.token_online = ''
    }
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '注册失败') }
  finally { registerLoading.value = false }
}

// ==================== 配置弹框 ====================
const showModal = ref(false)
const activeTab = ref('notify')

const config = reactive<any>({
  notify_enabled: 0, notify_type: '', notify_threshold: 5120,
  query_interval: 30, notify_title: '联通流量提醒', notify_subtitle: '',
  notify_content: '', nickname: '', query_password: '', auth_type: 'token_online',
  appid: '', token_online: '', cookie: ''
})
const notifyParams = reactive<Record<string, string>>({})

const notifyFieldMap: Record<string, any[]> = {
  telegram: [
    { name: 'bot_token', label: 'Bot Token', placeholder: '123456:ABC-xxx' },
    { name: 'chat_id', label: 'Chat ID', placeholder: '123456789' },
    { name: 'api_host', label: 'API 地址（选填）', placeholder: 'https://api.telegram.org' },
  ],
  wecom: [{ name: 'webhook', label: 'Webhook URL', placeholder: 'https://qyapi.weixin.qq.com/...' }],
  serverchan: [{ name: 'key', label: 'SendKey', placeholder: 'SCTxxxxx' }],
  dingtalk: [
    { name: 'webhook', label: 'Webhook URL', placeholder: 'https://oapi.dingtalk.com/...' },
    { name: 'secret', label: 'Secret (可选)', placeholder: 'SECxxx' },
  ],
  pushplus: [{ name: 'token', label: 'Token', placeholder: 'xxx' }],
}
const notifyFields = computed(() => notifyFieldMap[config.notify_type] || [])

function onNotifyTypeChange() {
  Object.keys(notifyParams).forEach(k => delete notifyParams[k])
}

function closeModal() { showModal.value = false }

async function openConfigModal() {
  const token = userInfo.query_token
  if (!token) { ElMessage.error('未找到用户令牌'); return }
  loading.value = true
  try {
    const res = await axios.get(`/user/${token}/config`)
    if (!res.data.success) { ElMessage.error('加载配置失败'); return }
    const c = res.data.data
    Object.assign(config, {
      notify_enabled: c.notify_enabled || 0, notify_type: c.notify_type || '',
      notify_threshold: c.notify_threshold || 5120, query_interval: c.query_interval || 30,
      notify_title: c.notify_title || '联通流量提醒', notify_subtitle: c.notify_subtitle || '',
      notify_content: c.notify_content || '', nickname: c.nickname || '',
      query_password: c.query_password || '', auth_type: c.auth_type || 'token_online',
      appid: c.appid || '', token_online: c.token_online || '', cookie: c.cookie || '',
    })
    try {
      const np = c.notify_params
      Object.assign(notifyParams, typeof np === 'string' ? JSON.parse(np || '{}') : (np || {}))
    } catch { /* ignore */ }
    activeTab.value = 'notify'
    showModal.value = true
  } catch (e: any) {
    ElMessage.error('加载配置失败: ' + (e.response?.data?.error || e.message))
  } finally { loading.value = false }
}

async function saveConfig() {
  const token = userInfo.query_token
  if (!token) return
  loading.value = true
  try {
    const body: any = { ...config, notify_params: JSON.stringify(notifyParams) }
    const res = await axios.post(`/user/${token}/config`, body)
    if (res.data.success) {
      ElMessage.success('配置已保存')
      // 更新本地昵称
      if (config.nickname) {
        userInfo.nickname = config.nickname
        localStorage.setItem('user_nickname', config.nickname)
      }
      closeModal()
    } else {
      ElMessage.error('保存失败: ' + (res.data.error || ''))
    }
  } catch (e: any) {
    ElMessage.error('保存失败: ' + (e.response?.data?.error || e.message))
  } finally { loading.value = false }
}

async function testNotify() {
  const token = userInfo.query_token
  if (!token) return
  loading.value = true
  try {
    const res = await axios.post(`/user/${token}/test-notify`)
    if (res.data.success) {
      ElMessage.success('测试通知已发送')
    } else {
      ElMessage.error('发送失败: ' + (res.data.error || ''))
    }
  } catch (e: any) {
    ElMessage.error('发送失败: ' + (e.response?.data?.error || e.message))
  } finally { loading.value = false }
}

// ==================== 初始化 ====================
const fetchGuestRegisterStatus = async () => {
  try {
    const res = await axios.get('/admin/system')
    if (res.data.success && res.data.data) {
      guestRegisterEnabled.value = res.data.data.guest_register_enabled === 1
    }
  } catch { /* 忽略，未登录时可能无权限，默认关闭 */ }
}

onMounted(() => {
  fetchGuestRegisterStatus()
})
</script>

<style scoped>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');

.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background: #f5f0e8;
  position: relative;
  overflow: hidden;
  font-family: 'Inter', sans-serif;
  color: #2c2c2c;
}

/* 背景装饰 */
.bg-decoration {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.circle {
  position: absolute;
  border-radius: 2px;
  background: rgba(180, 71, 47, 0.08);
  border: 2px dashed rgba(180, 71, 47, 0.15);
}

.circle-1 {
  width: 300px;
  height: 300px;
  top: -50px;
  right: -50px;
  animation: float 6s ease-in-out infinite;
}

.circle-2 {
  width: 200px;
  height: 200px;
  bottom: -30px;
  left: -30px;
  animation: float 8s ease-in-out infinite reverse;
}

.circle-3 {
  width: 150px;
  height: 150px;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  animation: float 10s ease-in-out infinite;
}

@keyframes float {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  50% { transform: translateY(-20px) rotate(5deg); }
}

/* 复古卡片效果 */
.glass-effect {
  background: #faf5ed;
  backdrop-filter: none;
  border: 2px dashed #c4b8a8;
}

.login-card {
  width: 440px;
  padding: 40px;
  border-radius: 2px;
  box-shadow: 4px 4px 0 #c4b8a8;
  position: relative;
  z-index: 1;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.login-card:hover {
  transform: translate(-1px, -1px);
  box-shadow: 5px 5px 0 #b4472f;
}

.login-header {
  text-align: center;
  margin-bottom: 30px;
}

.logo-area {
  margin-bottom: 16px;
}

.logo-icon {
  font-size: 48px;
  display: inline-block;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.05); }
}

.login-header h2 {
  color: #2c2c2c;
  margin-bottom: 8px;
  font-size: 28px;
  font-weight: 700;
  background: none;
  -webkit-background-clip: unset;
  -webkit-text-fill-color: unset;
  background-clip: unset;
}

.subtitle {
  color: #7a7067;
  font-size: 14px;
  margin: 0;
}

/* 用户信息区域 */
.user-info-section {
  background: #f0ebe3;
  border-radius: 2px;
  padding: 20px;
  margin-bottom: 24px;
  border: 2px dashed #c4b8a8;
}

.info-item {
  display: flex;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px dashed rgba(180, 71, 47, 0.15);
}

.info-item:last-child {
  border-bottom: none;
}

.info-icon {
  font-size: 20px;
  margin-right: 12px;
  width: 24px;
  text-align: center;
}

.info-content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.info-label {
  font-size: 12px;
  color: #7a7067;
  margin-bottom: 4px;
  font-family: 'JetBrains Mono', monospace;
}

.info-value {
  font-size: 16px;
  color: #2c2c2c;
  font-weight: 500;
}

.token-cell {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.token-tag {
  font-family: 'JetBrains Mono', monospace;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-btn {
  padding: 4px 8px;
  font-size: 12px;
  flex-shrink: 0;
}

/* 按钮样式 */
.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.login-btn {
  width: 100%;
  height: 48px;
  font-size: 16px;
  font-weight: 600;
  border-radius: 2px;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-family: 'Inter', sans-serif;
}

.primary-btn {
  background: #b4472f;
  border: 2px dashed #8a3623;
  color: #faf5ed;
  box-shadow: 3px 3px 0 #8a3623;
}

.primary-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #6b2a1b;
  background: #c9543a;
}

.primary-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #8a3623;
}

.secondary-btn {
  background: transparent;
  border: 2px dashed #b4472f;
  color: #b4472f;
  box-shadow: 3px 3px 0 rgba(180, 71, 47, 0.3);
}

.secondary-btn:hover {
  background: #f0ebe3;
  color: #b4472f;
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 rgba(180, 71, 47, 0.4);
}

.secondary-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 rgba(180, 71, 47, 0.3);
}

.success-btn {
  background: #4a7c59;
  border: 2px dashed #3a6347;
  color: #faf5ed;
  box-shadow: 3px 3px 0 #3a6347;
}

.success-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #2d4e38;
  background: #568d66;
}

.success-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #3a6347;
}

.btn-icon {
  font-size: 18px;
}

/* 表单样式 */
.login-form {
  margin-top: 10px;
}

.custom-input {
  border-radius: 2px;
}

.custom-input :deep(.el-input__wrapper) {
  border-radius: 2px;
  box-shadow: 2px 2px 0 #c4b8a8;
  border: 2px dashed #c4b8a8;
  background: #faf5ed;
  transition: all 0.15s ease;
}

.custom-input :deep(.el-input__wrapper:hover) {
  box-shadow: 3px 3px 0 #b4472f;
  border-color: #b4472f;
}

.custom-input :deep(.el-input__wrapper.is-focus) {
  box-shadow: 3px 3px 0 #b4472f;
  border-color: #b4472f;
}

/* Tab 样式 */
.auth-tabs {
  margin-top: 5px;
}

.auth-tabs :deep(.el-tabs__header) {
  margin-bottom: 20px;
}

.auth-tabs :deep(.el-tabs__nav-wrap::after) {
  display: none;
}

.auth-tabs :deep(.el-tabs__item) {
  font-size: 15px;
  font-weight: 600;
  color: #7a7067;
  transition: all 0.15s ease;
  font-family: 'Inter', sans-serif;
}

.auth-tabs :deep(.el-tabs__item.is-active) {
  color: #b4472f;
}

.auth-tabs :deep(.el-tabs__active-bar) {
  background: #b4472f;
  height: 3px;
  border-radius: 2px;
}

/* 分隔线 */
.divider-section {
  display: flex;
  align-items: center;
  margin: 20px 0;
  gap: 12px;
}

.divider-line {
  flex: 1;
  height: 0;
  border-top: 2px dashed #c4b8a8;
}

.divider-text {
  color: #7a7067;
  font-size: 12px;
  white-space: nowrap;
  font-family: 'JetBrains Mono', monospace;
}

/* 响应式 */
@media (max-width: 480px) {
  .login-card {
    width: calc(100% - 32px);
    padding: 24px 16px;
    margin: 16px;
  }

  .login-header h2 {
    font-size: 22px;
  }

  .logo-icon {
    font-size: 36px;
  }

  .info-item {
    align-items: flex-start;
  }

  .info-icon {
    margin-right: 10px;
    font-size: 18px;
  }

  .info-value {
    font-size: 14px;
    word-break: break-all;
  }

  .token-cell {
    flex-wrap: wrap;
  }

  .token-tag {
    font-size: 11px;
  }

  .action-buttons .login-btn {
    font-size: 14px;
  }
}

/* 设置按钮 */
.warning-btn {
  background: #d4952a;
  border: 2px dashed #b07a20;
  color: #faf5ed;
  box-shadow: 3px 3px 0 #b07a20;
}

.warning-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #8a6118;
  background: #e0a335;
}

.warning-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #b07a20;
}

.action-buttons-2x2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.danger-btn {
  background: #c0392b;
  border: 2px dashed #962d22;
  color: #faf5ed;
  box-shadow: 3px 3px 0 #962d22;
}

.danger-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #7a241b;
  background: #d44637;
}

.danger-btn:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #962d22;
}

/* 配置弹框 */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(44, 44, 44, 0.5);
  display: none;
  justify-content: center;
  align-items: center;
  z-index: 2000;
  padding: 20px;
}

.modal-overlay.show {
  display: flex;
}

.modal {
  background: #faf5ed;
  border-radius: 2px;
  width: 100%;
  max-width: 500px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 6px 6px 0 #c4b8a8;
  border: 2px dashed #c4b8a8;
  font-family: 'Inter', sans-serif;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 2px dashed #c4b8a8;
}

.modal-tabs {
  display: flex;
  gap: 4px;
}

.modal-tabs button {
  padding: 8px 16px;
  border: 2px dashed #c4b8a8;
  background: #f0ebe3;
  border-radius: 2px;
  cursor: pointer;
  font-size: 14px;
  color: #7a7067;
  transition: all 0.15s ease;
  box-shadow: 2px 2px 0 #c4b8a8;
  font-family: 'Inter', sans-serif;
}

.modal-tabs button:hover {
  box-shadow: 3px 3px 0 #b4472f;
  border-color: #b4472f;
  color: #b4472f;
}

.modal-tabs button.active {
  background: #b4472f;
  color: #faf5ed;
  border-color: #8a3623;
  box-shadow: 2px 2px 0 #8a3623;
}

.modal-close {
  width: 32px;
  height: 32px;
  border: 2px dashed #c4b8a8;
  background: #f0ebe3;
  border-radius: 2px;
  cursor: pointer;
  font-size: 16px;
  color: #7a7067;
  transition: all 0.15s ease;
  box-shadow: 2px 2px 0 #c4b8a8;
}

.modal-close:hover {
  background: #b4472f;
  color: #faf5ed;
  border-color: #8a3623;
  box-shadow: 2px 2px 0 #8a3623;
}

.modal-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 16px;
}

.form-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: #2c2c2c;
  margin-bottom: 6px;
  font-family: 'JetBrains Mono', monospace;
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 2px dashed #c4b8a8;
  border-radius: 2px;
  font-size: 14px;
  box-sizing: border-box;
  transition: border-color 0.15s;
  background: #faf5ed;
  color: #2c2c2c;
  font-family: 'Inter', sans-serif;
  box-shadow: 2px 2px 0 #c4b8a8;
}

.form-input:focus {
  outline: none;
  border-color: #b4472f;
  box-shadow: 3px 3px 0 #b4472f;
}

textarea.form-input {
  resize: vertical;
  font-family: 'JetBrains Mono', monospace;
}

.form-hint {
  font-size: 11px;
  color: #7a7067;
  margin-top: 4px;
  font-family: 'JetBrains Mono', monospace;
}

.modal-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 20px;
}

.btn-primary {
  background: #b4472f;
  color: #faf5ed;
  border: 2px dashed #8a3623;
  padding: 10px 24px;
  border-radius: 2px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.15s ease;
  box-shadow: 3px 3px 0 #8a3623;
  font-family: 'Inter', sans-serif;
}

.btn-primary:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 #6b2a1b;
  background: #c9543a;
}

.btn-primary:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #8a3623;
}

.btn-secondary {
  background: #f0ebe3;
  color: #7a7067;
  border: 2px dashed #c4b8a8;
  padding: 10px 24px;
  border-radius: 2px;
  cursor: pointer;
  font-size: 14px;
  box-shadow: 2px 2px 0 #c4b8a8;
  font-family: 'Inter', sans-serif;
}

.btn-secondary:hover {
  border-color: #b4472f;
  color: #b4472f;
  box-shadow: 3px 3px 0 #b4472f;
}

select.form-input {
  appearance: auto;
}

input[type="checkbox"] {
  margin-right: 6px;
}
</style>
