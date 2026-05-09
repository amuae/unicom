<template>
  <div class="admin-container">
    <!-- Admin Login Card -->
    <div v-if="!isAdminLoggedIn" class="login-container">
      <div class="bg-decoration">
        <div class="circle circle-1"></div>
        <div class="circle circle-2"></div>
        <div class="circle circle-3"></div>
      </div>
      <div class="login-card glass-effect">
        <div class="login-header">
          <div class="logo-area">
            <span class="logo-icon">📱</span>
          </div>
          <h2>联通流量查询系统</h2>
          <p class="subtitle">管理后台</p>
        </div>
        <el-form :model="adminForm" :rules="adminRules" ref="adminFormRef" class="login-form">
          <el-form-item prop="username">
            <el-input v-model="adminForm.username" placeholder="请输入用户名" prefix-icon="User" size="large" class="custom-input" />
          </el-form-item>
          <el-form-item prop="password">
            <el-input v-model="adminForm.password" type="password" placeholder="请输入密码" prefix-icon="Lock" size="large" show-password @keyup.enter="handleAdminLogin" class="custom-input" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleAdminLogin" :loading="submitLoading" size="large" class="login-btn primary-btn">
              <span class="btn-icon">🔐</span> 登录
            </el-button>
          </el-form-item>
        </el-form>
      </div>
    </div>

    <!-- 顶部导航 -->
    <el-header v-if="isAdminLoggedIn" class="admin-header">
      <div class="header-left">
        <div class="logo-section">
          <span class="logo-icon">📱</span>
          <h1>联通流量查询系统</h1>
        </div>
        <span class="admin-badge">管理后台</span>
      </div>
      <div class="header-right">
        <div class="user-info">
          <span class="user-avatar">👤</span>
          <span class="username">{{ username }}</span>
        </div>
        <el-button type="danger" text @click="handleLogout" class="logout-btn">
          <span class="btn-icon">🚪</span> 退出登录
        </el-button>
      </div>
    </el-header>

    <!-- 主内容区 -->
    <el-main v-if="isAdminLoggedIn" class="admin-main">
      <el-tabs v-model="activeTab" @tab-click="handleTabClick" class="admin-tabs">
        <!-- ==================== Tab 1: 用户管理 ==================== -->
        <el-tab-pane label="用户管理" name="users">
          <div class="tab-header">
            <div class="tab-title">
              <span class="tab-icon">👥</span>
              <span>用户列表</span>
            </div>
            <div class="tab-actions">
              <el-button type="primary" @click="showAddUserDialog" class="action-btn">
                <span class="btn-icon">➕</span> 添加用户
              </el-button>
              <el-button @click="fetchUsers" class="action-btn">
                <span class="btn-icon">🔄</span> 刷新
              </el-button>
            </div>
          </div>
          <el-table :data="users" border stripe class="user-table modern-table">
            <el-table-column prop="id" label="ID" width="50" />
            <el-table-column prop="mobile" label="手机号" width="110" />
            <el-table-column prop="nickname" label="昵称" width="80" />
            <el-table-column label="查询链接" min-width="160">
              <template #default="{ row }">
                <div v-if="row.token" class="token-cell" @click="copyLink(row.token)" title="点击复制">
                  <el-tag size="small" type="info" class="token-tag">{{ row.token }}</el-tag>
                  <span class="copy-hint">📋</span>
                </div>
                <span v-else style="color:#999">未生成</span>
              </template>
            </el-table-column>
            <el-table-column label="通知" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.notify_enabled" :type="row.notify_type ? 'success' : 'warning'" size="small">
                  {{ row.notify_type || '未配置' }}
                </el-tag>
                <el-tag v-else type="info" size="small">关闭</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="query_interval" label="间隔" width="60">
              <template #default="{ row }">{{ row.query_interval || 30 }}m</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="70">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'danger'" size="small">
                  {{ row.status === 'active' ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="last_query_at" label="最后查询" width="155" />
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button size="small" @click="showEditUserDialog(row)" class="table-btn">编辑</el-button>
                <el-button size="small" @click="openNotifyDialog(row)" class="table-btn">通知</el-button>
                <el-button size="small" :type="row.status === 'active' ? 'warning' : 'success'" @click="toggleUserStatus(row.id)" class="table-btn">
                  {{ row.status === 'active' ? '禁用' : '启用' }}
                </el-button>
                <el-button size="small" type="danger" @click="deleteUser(row.id)" class="table-btn">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- ==================== Tab 2: 定时管理 ==================== -->
        <el-tab-pane label="定时管理" name="cron">
          <div class="tab-header">
            <el-button type="primary" @click="showAddCronDialog">添加定时任务</el-button>
            <el-button @click="fetchCron">刷新</el-button>
          </div>
          <el-table :data="cronList" border stripe>
            <el-table-column prop="id" label="ID" width="60" />
            <el-table-column prop="mobile" label="手机号" width="120" />
            <el-table-column prop="nickname" label="昵称" width="100" />
            <el-table-column prop="cron_expression" label="Cron表达式" width="180" />
            <el-table-column prop="status" label="状态" width="90">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="last_run_at" label="上次执行" width="180" />
            <el-table-column label="成功/失败" width="100">
              <template #default="{ row }">
                <span style="color:#67c23a">{{ row.success_runs || 0 }}</span> / <span style="color:#f56c6c">{{ row.failed_runs || 0 }}</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="160" fixed="right">
              <template #default="{ row }">
                <el-button size="small" :type="row.status === 'active' ? 'warning' : 'success'" @click="toggleCron(row.id)">
                  {{ row.status === 'active' ? '禁用' : '启用' }}
                </el-button>
                <el-button size="small" type="danger" @click="deleteCron(row.id)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- ==================== Tab 3: 日志管理 ==================== -->
        <el-tab-pane label="日志管理" name="logs">
          <div class="tab-header">
            <el-select v-model="logFilter.type" @change="fetchLogs" style="width: 120px;" clearable placeholder="日志类型">
              <el-option label="全部" value="" />
              <el-option label="系统" value="system" />
              <el-option label="查询" value="query" />
              <el-option label="定时" value="cron" />
              <el-option label="认证" value="auth" />
            </el-select>
            <el-select v-model="logFilter.level" @change="fetchLogs" style="width: 120px;" clearable placeholder="日志级别">
              <el-option label="全部" value="" />
              <el-option label="Info" value="info" />
              <el-option label="Warn" value="warn" />
              <el-option label="Error" value="error" />
            </el-select>
            <el-button @click="fetchLogs">刷新</el-button>
            <div style="flex:1"></div>
            <el-button @click="showLogConfigDialog">日志配置</el-button>
          </div>
          <el-table :data="logs" border stripe>
            <el-table-column prop="id" label="ID" width="60" />
            <el-table-column prop="log_type" label="类型" width="80">
              <template #default="{ row }">
                <el-tag size="small">{{ row.log_type || '-' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="log_level" label="级别" width="80">
              <template #default="{ row }">
                <el-tag :type="getLogLevelType(row.log_level)" size="small">{{ row.log_level }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="message" label="消息" min-width="200" show-overflow-tooltip />
            <el-table-column prop="created_at" label="时间" width="180" />
          </el-table>
        </el-tab-pane>

        <!-- ==================== Tab 4: 系统管理 ==================== -->
        <el-tab-pane label="系统管理" name="system">
          <div class="system-section">
            <el-card shadow="never">
              <template #header><span>修改管理员密码</span></template>
              <el-form :model="passwordForm" label-width="100px" style="max-width: 400px;">
                <el-form-item label="旧密码">
                  <el-input v-model="passwordForm.old_password" type="password" show-password placeholder="请输入旧密码" />
                </el-form-item>
                <el-form-item label="新密码">
                  <el-input v-model="passwordForm.new_password" type="password" show-password placeholder="请输入新密码" />
                </el-form-item>
                <el-form-item label="确认密码">
                  <el-input v-model="passwordForm.confirm_password" type="password" show-password placeholder="请再次输入新密码" />
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" @click="changePassword" :loading="submitLoading">保存</el-button>
                </el-form-item>
              </el-form>
            </el-card>

            <el-card shadow="never" style="margin-top: 20px;">
              <template #header><span>系统配置</span></template>
              <el-form label-width="160px" style="max-width: 500px;">
                <el-form-item label="游客注册">
                  <el-switch v-model="systemConfig.guest_register_enabled" :active-value="1" :inactive-value="0" @change="toggleGuestRegister" />
                  <span style="margin-left: 10px; color: #999;">{{ systemConfig.guest_register_enabled ? '已开启' : '已关闭' }}</span>
                </el-form-item>
              </el-form>
            </el-card>

            <el-card shadow="never" style="margin-top: 20px;">
              <template #header><span>修改管理员用户名</span></template>
              <el-form :model="usernameForm" label-width="100px" style="max-width: 400px;">
                <el-form-item label="新用户名">
                  <el-input v-model="usernameForm.new_username" placeholder="请输入新用户名" />
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" @click="changeUsername" :loading="submitLoading">保存</el-button>
                </el-form-item>
              </el-form>
            </el-card>
          </div>
        </el-tab-pane>
      </el-tabs>
    </el-main>

    <!-- 添加用户对话框 -->
    <el-dialog v-model="addUserVisible" title="添加用户" width="500">
      <el-form :model="addUserForm" :rules="addUserRules" ref="addUserFormRef" label-width="100px">
        <el-form-item label="手机号" prop="mobile">
          <el-input v-model="addUserForm.mobile" placeholder="请输入手机号" />
        </el-form-item>
        <el-form-item label="昵称">
          <el-input v-model="addUserForm.nickname" placeholder="请输入昵称（可选）" />
        </el-form-item>
        <el-form-item label="查询密码" prop="query_password">
          <el-input v-model="addUserForm.query_password" placeholder="请输入查询密码" />
        </el-form-item>
        <el-form-item label="认证方式" prop="auth_type">
          <el-radio-group v-model="addUserForm.auth_type">
            <el-radio label="cookie">Cookie</el-radio>
            <el-radio label="token_online">Token Online</el-radio>
          </el-radio-group>
        </el-form-item>
        <template v-if="addUserForm.auth_type === 'cookie'">
          <el-form-item label="Cookie" prop="cookie">
            <el-input v-model="addUserForm.cookie" type="textarea" :rows="4" placeholder="请输入联通 Cookie" />
          </el-form-item>
        </template>
        <template v-if="addUserForm.auth_type === 'token_online'">
          <el-form-item label="App ID" prop="appid">
            <el-input v-model="addUserForm.appid" placeholder="请输入 App ID" />
          </el-form-item>
          <el-form-item label="Token Online" prop="token_online">
            <el-input v-model="addUserForm.token_online" type="textarea" :rows="3" placeholder="请输入 Token Online" />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="addUserVisible = false">取消</el-button>
        <el-button type="primary" @click="submitAddUser" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <!-- 编辑用户对话框 -->
    <el-dialog v-model="editUserVisible" title="编辑用户" width="500">
      <el-form :model="editUserForm" ref="editUserFormRef" label-width="100px">
        <el-form-item label="手机号">
          <el-input v-model="editUserForm.mobile" />
        </el-form-item>
        <el-form-item label="昵称">
          <el-input v-model="editUserForm.nickname" />
        </el-form-item>
        <el-form-item label="查询密码">
          <el-input v-model="editUserForm.query_password" placeholder="留空则不修改" />
        </el-form-item>
        <el-form-item label="认证方式">
          <el-radio-group v-model="editUserForm.auth_type">
            <el-radio label="cookie">Cookie</el-radio>
            <el-radio label="token_online">Token Online</el-radio>
          </el-radio-group>
        </el-form-item>
        <template v-if="editUserForm.auth_type === 'cookie'">
          <el-form-item label="Cookie">
            <el-input v-model="editUserForm.cookie" type="textarea" :rows="4" placeholder="留空则不修改" />
          </el-form-item>
        </template>
        <template v-if="editUserForm.auth_type === 'token_online'">
          <el-form-item label="App ID">
            <el-input v-model="editUserForm.appid" placeholder="留空则不修改" />
          </el-form-item>
          <el-form-item label="Token Online">
            <el-input v-model="editUserForm.token_online" type="textarea" :rows="3" placeholder="留空则不修改" />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="editUserVisible = false">取消</el-button>
        <el-button type="primary" @click="submitEditUser" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <!-- 通知配置对话框 -->
    <el-dialog v-model="notifyVisible" title="通知配置" width="500">
      <el-form :model="notifyForm" label-width="100px">
        <el-form-item label="启用通知">
          <el-switch v-model="notifyForm.notify_enabled" :active-value="1" :inactive-value="0" />
        </el-form-item>
        <el-form-item label="通知渠道">
          <el-select v-model="notifyForm.notify_type" @change="onNotifyTypeChange">
            <el-option label="请选择" value="" />
            <el-option label="Telegram" value="telegram" />
            <el-option label="企业微信" value="wecom" />
            <el-option label="Server酱" value="serverchan" />
            <el-option label="钉钉" value="dingtalk" />
            <el-option label="PushPlus" value="pushplus" />
          </el-select>
        </el-form-item>
        <template v-if="notifyForm.notify_type === 'telegram'">
          <el-form-item label="Bot Token"><el-input v-model="notifyParams.bot_token" placeholder="123456:ABC-xxx" /></el-form-item>
          <el-form-item label="Chat ID"><el-input v-model="notifyParams.chat_id" placeholder="123456789" /></el-form-item>
          <el-form-item label="API 地址">
            <el-input v-model="notifyParams.api_host" placeholder="留空默认 api.telegram.org" />
            <div style="color: #999; font-size: 12px; margin-top: 4px;">自建 Bot API 服务地址，如 https://tg.suuus.de</div>
          </el-form-item>
        </template>
        <template v-if="notifyForm.notify_type === 'wecom'">
          <el-form-item label="Webhook"><el-input v-model="notifyParams.webhook" placeholder="https://qyapi.weixin.qq.com/..." /></el-form-item>
        </template>
        <template v-if="notifyForm.notify_type === 'serverchan'">
          <el-form-item label="SendKey"><el-input v-model="notifyParams.key" placeholder="SCTxxxxx" /></el-form-item>
        </template>
        <template v-if="notifyForm.notify_type === 'dingtalk'">
          <el-form-item label="Webhook"><el-input v-model="notifyParams.webhook" placeholder="https://oapi.dingtalk.com/..." /></el-form-item>
          <el-form-item label="Secret"><el-input v-model="notifyParams.secret" placeholder="SECxxx（可选）" /></el-form-item>
        </template>
        <template v-if="notifyForm.notify_type === 'pushplus'">
          <el-form-item label="Token"><el-input v-model="notifyParams.token" placeholder="xxx" /></el-form-item>
        </template>
        <el-form-item label="阈值 (MB)">
          <el-input-number v-model="notifyForm.notify_threshold" :min="0" :step="512" />
          <div class="form-hint">通用流量用量达到此值时通知，0=每次通知</div>
        </el-form-item>
        <el-form-item label="查询间隔">
          <el-input-number v-model="notifyForm.query_interval" :min="5" :step="5" />
          <span style="margin-left:8px;color:#999">分钟</span>
        </el-form-item>
        <el-form-item label="通知标题">
          <el-input v-model="notifyForm.notify_title" placeholder="联通流量提醒" />
        </el-form-item>
        <el-form-item label="通知内容">
          <el-input v-model="notifyForm.notify_content" type="textarea" :rows="4" placeholder="支持占位符: [套餐] [时间] [时长] [桶名.用量]" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="testNotify" :loading="testLoading">测试通知</el-button>
        <el-button @click="notifyVisible = false">取消</el-button>
        <el-button type="primary" @click="submitNotify" :loading="submitLoading">保存</el-button>
      </template>
    </el-dialog>

    <!-- 添加定时任务对话框 -->
    <el-dialog v-model="addCronVisible" title="添加定时任务" width="450">
      <el-form :model="addCronForm" label-width="100px">
        <el-form-item label="选择用户">
          <el-select v-model="addCronForm.user_id" filterable placeholder="请选择用户" style="width: 100%;">
            <el-option v-for="u in users" :key="u.id" :label="`${u.mobile} (${u.nickname || '-'})`" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="Cron表达式">
          <el-input v-model="addCronForm.cron_expression" placeholder="例: 0 8 * * * (每天8点)" />
          <div class="form-hint">格式: 分 时 日 月 周</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="addCronVisible = false">取消</el-button>
        <el-button type="primary" @click="submitAddCron" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <!-- 日志配置对话框 -->
    <el-dialog v-model="logConfigVisible" title="日志配置" width="400">
      <el-form :model="logConfig" label-width="100px">
        <el-form-item label="日志级别">
          <el-select v-model="logConfig.log_level" style="width: 100%;">
            <el-option label="Debug" value="debug" />
            <el-option label="Info" value="info" />
            <el-option label="Warn" value="warn" />
            <el-option label="Error" value="error" />
          </el-select>
        </el-form-item>
        <el-form-item label="保留天数">
          <el-input-number v-model="logConfig.log_retention_days" :min="1" :max="365" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button type="warning" @click="cleanLogs" :loading="cleanLoading">清理日志</el-button>
        <el-button @click="logConfigVisible = false">取消</el-button>
        <el-button type="primary" @click="saveLogConfig" :loading="submitLoading">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import axios from 'axios'

const router = useRouter()
const activeTab = ref('users')
const username = ref(localStorage.getItem('username') || '管理员')
const submitLoading = ref(false)
const testLoading = ref(false)
const cleanLoading = ref(false)

// ==================== Admin Login ====================
const adminLoggedIn = ref(!!localStorage.getItem('admin_token'))
const isAdminLoggedIn = computed(() => adminLoggedIn.value || !!localStorage.getItem('admin_token'))
const adminFormRef = ref()
const adminForm = reactive({ username: '', password: '' })
const adminRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
}
const handleAdminLogin = async () => {
  if (!adminFormRef.value) { ElMessage.error('表单未加载，请刷新页面'); return }
  try { await adminFormRef.value.validate() } catch { return }
  submitLoading.value = true
  try {
    const res = await axios.post('/auth/login', adminForm)
    if (res.data.token) {
      localStorage.setItem('admin_token', res.data.token)
      localStorage.setItem('token', res.data.token)
      localStorage.setItem('username', res.data.username || adminForm.username)
      username.value = res.data.username || adminForm.username
      adminLoggedIn.value = true
      ElMessage.success('登录成功')
      fetchUsers()
    }
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '登录失败') }
  finally { submitLoading.value = false }
}

const authHeaders = () => ({ headers: { Authorization: `Bearer ${localStorage.getItem('token')}` } })

// ==================== 用户管理 ====================
const users = ref<any[]>([])
const fetchUsers = async () => {
  try {
    const res = await axios.get('/admin/users', authHeaders())
    if (res.data.success) users.value = res.data.data || []
  } catch (e) { console.error('获取用户列表失败:', e) }
}

const addUserVisible = ref(false)
const addUserFormRef = ref()
const addUserForm = ref({ mobile: '', nickname: '', query_password: '', auth_type: 'cookie', cookie: '', appid: '', token_online: '' })
const addUserRules = {
  mobile: [{ required: true, message: '请输入手机号', trigger: 'blur' }, { pattern: /^1[3-9]\d{9}$/, message: '手机号格式不正确', trigger: 'blur' }],
  query_password: [{ required: true, message: '请输入查询密码', trigger: 'blur' }],
  auth_type: [{ required: true, message: '请选择认证方式', trigger: 'change' }],
}
const showAddUserDialog = () => {
  addUserForm.value = { mobile: '', nickname: '', query_password: '', auth_type: 'cookie', cookie: '', appid: '', token_online: '' }
  addUserVisible.value = true
}
const submitAddUser = async () => {
  if (!addUserFormRef.value) { ElMessage.error('表单未加载，请刷新页面'); return }
  try {
    await addUserFormRef.value.validate()
  } catch { return }
  if (addUserForm.value.auth_type === 'cookie' && !addUserForm.value.cookie) { ElMessage.error('请输入 Cookie'); return }
  if (addUserForm.value.auth_type === 'token_online' && (!addUserForm.value.appid || !addUserForm.value.token_online)) { ElMessage.error('请输入 App ID 和 Token Online'); return }
  submitLoading.value = true
  try {
    const res = await axios.post('/admin/users', { ...addUserForm.value }, authHeaders())
    if (res.data.success) { ElMessage.success('添加成功'); addUserVisible.value = false; fetchUsers() }
    else ElMessage.error(res.data.error || '添加失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '添加失败') }
  finally { submitLoading.value = false }
}

const editUserVisible = ref(false)
const editUserFormRef = ref()
const editUserForm = ref({ id: 0, mobile: '', nickname: '', query_password: '', auth_type: 'cookie', cookie: '', appid: '', token_online: '' })
const showEditUserDialog = async (user: any) => {
  editUserForm.value = { id: user.id, mobile: user.mobile, nickname: user.nickname, query_password: '', auth_type: user.auth_type, cookie: '', appid: '', token_online: '' }
  editUserVisible.value = true
  try {
    const token = localStorage.getItem('token')
    const res = await axios.get(`/admin/users/${user.id}`, { headers: { Authorization: `Bearer ${token}` } })
    if (res.data.success) {
      const d = res.data.data
      editUserForm.value.query_password = d.query_password || ''
      editUserForm.value.appid = d.appid || ''
      editUserForm.value.token_online = d.token_online || ''
      editUserForm.value.cookie = d.cookie || ''
    }
  } catch { /* ignore */ }
}
const submitEditUser = async () => {
  submitLoading.value = true
  try {
    const f = editUserForm.value
    const updateData: any = { id: f.id }
    if (f.mobile) updateData.mobile = f.mobile
    if (f.nickname) updateData.nickname = f.nickname
    if (f.query_password) updateData.query_password = f.query_password
    if (f.auth_type) updateData.auth_type = f.auth_type
    if (f.cookie) updateData.cookie = f.cookie
    if (f.appid) updateData.appid = f.appid
    if (f.token_online) updateData.token_online = f.token_online
    const res = await axios.put('/admin/users', updateData, authHeaders())
    if (res.data.success) { ElMessage.success('更新成功'); editUserVisible.value = false; fetchUsers() }
    else ElMessage.error(res.data.error || '更新失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '更新失败') }
  finally { submitLoading.value = false }
}

const notifyVisible = ref(false)
const notifyUserId = ref(0)
const notifyForm = reactive({ notify_enabled: 0, notify_type: '', notify_threshold: 5120, query_interval: 30, notify_title: '联通流量提醒', notify_content: '' })
const notifyParams = reactive<Record<string, string>>({})

const openNotifyDialog = (user: any) => {
  notifyUserId.value = user.id
  notifyForm.notify_enabled = user.notify_enabled || 0
  notifyForm.notify_type = user.notify_type || ''
  notifyForm.notify_threshold = 5120
  notifyForm.query_interval = user.query_interval || 30
  notifyForm.notify_title = '联通流量提醒'
  notifyForm.notify_content = ''
  Object.keys(notifyParams).forEach(k => delete notifyParams[k])
  loadUserNotifyConfig(user.token)
  notifyVisible.value = true
}
const loadUserNotifyConfig = async (userToken: string) => {
  try {
    const res = await axios.get(`/user/${userToken}/config`)
    if (res.data.success) {
      const c = res.data.data
      notifyForm.notify_enabled = c.notify_enabled || 0
      notifyForm.notify_type = c.notify_type || ''
      notifyForm.notify_threshold = c.notify_threshold || 5120
      notifyForm.query_interval = c.query_interval || 30
      notifyForm.notify_title = c.notify_title || '联通流量提醒'
      notifyForm.notify_content = c.notify_content || ''
      try {
        const np = c.notify_params
        Object.assign(notifyParams, typeof np === 'string' ? JSON.parse(np || '{}') : (np || {}))
      } catch { /* ignore */ }
    }
  } catch { /* ignore */ }
}
const onNotifyTypeChange = () => { Object.keys(notifyParams).forEach(k => delete notifyParams[k]) }
const submitNotify = async () => {
  submitLoading.value = true
  try {
    const userToken = users.value.find(u => u.id === notifyUserId.value)?.token
    if (!userToken) { ElMessage.error('找不到用户'); return }
    const body = { ...notifyForm, notify_params: JSON.stringify(notifyParams) }
    const res = await axios.post(`/user/${userToken}/config`, body)
    if (res.data.success) { ElMessage.success('通知配置已保存'); notifyVisible.value = false; fetchUsers() }
    else ElMessage.error(res.data.error || '保存失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '保存失败') }
  finally { submitLoading.value = false }
}
const testNotify = async () => {
  testLoading.value = true
  try {
    const userToken = users.value.find(u => u.id === notifyUserId.value)?.token
    if (!userToken) { ElMessage.error('找不到用户'); return }
    const res = await axios.post(`/user/${userToken}/test-notify`)
    if (res.data.success) ElMessage.success('测试通知已发送')
    else ElMessage.error(res.data.error || '发送失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '发送失败') }
  finally { testLoading.value = false }
}

const toggleUserStatus = async (userId: number) => {
  try {
    const res = await axios.post('/admin/users/toggle', { id: userId }, authHeaders())
    if (res.data.success) { ElMessage.success(res.data.data.message); fetchUsers() }
    else ElMessage.error(res.data.error || '操作失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '操作失败') }
}
const deleteUser = async (userId: number) => {
  try {
    await ElMessageBox.confirm('确定要删除该用户吗？', '提示', { type: 'warning' })
    const res = await axios.delete('/admin/users', { ...authHeaders(), data: { id: userId } })
    if (res.data.success) { ElMessage.success('删除成功'); fetchUsers() }
    else ElMessage.error(res.data.error || '删除失败')
  } catch (e: any) { if (e !== 'cancel') ElMessage.error(e.response?.data?.error || '删除失败') }
}

const copyLink = (token: string) => {
  const url = `${window.location.origin}/query/${token}`
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

// ==================== 定时管理 ====================
const cronList = ref<any[]>([])
const addCronVisible = ref(false)
const addCronForm = ref({ user_id: null as number | null, cron_expression: '' })

const fetchCron = async () => {
  try {
    const res = await axios.get('/admin/cron', authHeaders())
    if (res.data.success) cronList.value = res.data.data || []
  } catch (e) { console.error('获取定时任务失败:', e) }
}
const showAddCronDialog = () => {
  addCronForm.value = { user_id: null, cron_expression: '' }
  if (users.value.length === 0) fetchUsers()
  addCronVisible.value = true
}
const submitAddCron = async () => {
  if (!addCronForm.value.user_id || !addCronForm.value.cron_expression) {
    ElMessage.error('请填写完整'); return
  }
  submitLoading.value = true
  try {
    const res = await axios.post('/admin/cron', addCronForm.value, authHeaders())
    if (res.data.success) { ElMessage.success('添加成功'); addCronVisible.value = false; fetchCron() }
    else ElMessage.error(res.data.error || '添加失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '添加失败') }
  finally { submitLoading.value = false }
}
const toggleCron = async (id: number) => {
  try {
    const res = await axios.post('/admin/cron/toggle', { id }, authHeaders())
    if (res.data.success) { ElMessage.success('操作成功'); fetchCron() }
    else ElMessage.error(res.data.error || '操作失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '操作失败') }
}
const deleteCron = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除该定时任务吗？', '提示', { type: 'warning' })
    const res = await axios.delete('/admin/cron', { ...authHeaders(), data: { id } })
    if (res.data.success) { ElMessage.success('删除成功'); fetchCron() }
    else ElMessage.error(res.data.error || '删除失败')
  } catch (e: any) { if (e !== 'cancel') ElMessage.error(e.response?.data?.error || '删除失败') }
}

// ==================== 日志管理 ====================
const logs = ref<any[]>([])
const logFilter = reactive({ type: '', level: '' })
const logConfigVisible = ref(false)
const logConfig = reactive({ log_level: 'info', log_retention_days: 30 })

const fetchLogs = async () => {
  try {
    const params = new URLSearchParams()
    if (logFilter.type) params.append('type', logFilter.type)
    if (logFilter.level) params.append('level', logFilter.level)
    params.append('limit', '200')
    const res = await axios.get(`/admin/logs?${params.toString()}`, authHeaders())
    if (res.data.success) {
      const d = res.data.data
      logs.value = Array.isArray(d) ? d : (d?.logs || [])
    }
  } catch (e) { console.error('获取日志失败:', e) }
}
const getLogLevelType = (level: string) => {
  switch (level) { case 'error': return 'danger'; case 'warn': return 'warning'; case 'info': return 'info'; default: return '' }
}
const showLogConfigDialog = async () => {
  try {
    const res = await axios.get('/admin/logs/config', authHeaders())
    if (res.data.success && res.data.data) {
      logConfig.log_level = res.data.data.log_level || 'info'
      logConfig.log_retention_days = res.data.data.log_retention_days || 30
    }
  } catch { /* ignore */ }
  logConfigVisible.value = true
}
const saveLogConfig = async () => {
  submitLoading.value = true
  try {
    const res = await axios.post('/admin/logs/config', logConfig, authHeaders())
    if (res.data.success) { ElMessage.success('配置已保存'); logConfigVisible.value = false }
    else ElMessage.error(res.data.error || '保存失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '保存失败') }
  finally { submitLoading.value = false }
}
const cleanLogs = async () => {
  try {
    await ElMessageBox.confirm('确定要清理过期日志吗？', '提示', { type: 'warning' })
    cleanLoading.value = true
    const res = await axios.post('/admin/logs/clean', {}, authHeaders())
    if (res.data.success) ElMessage.success('日志已清理')
    else ElMessage.error(res.data.error || '清理失败')
  } catch (e: any) { if (e !== 'cancel') ElMessage.error(e.response?.data?.error || '清理失败') }
  finally { cleanLoading.value = false }
}

// ==================== 系统管理 ====================
const systemConfig = reactive({ guest_register_enabled: 0 })
const passwordForm = reactive({ old_password: '', new_password: '', confirm_password: '' })

const fetchSystemConfig = async () => {
  try {
    const res = await axios.get('/admin/system', authHeaders())
    if (res.data.success && res.data.data) {
      systemConfig.guest_register_enabled = res.data.data.guest_register_enabled || 0
    }
  } catch { /* ignore */ }
}
const changePassword = async () => {
  if (!passwordForm.old_password || !passwordForm.new_password) { ElMessage.error('请填写完整'); return }
  if (passwordForm.new_password !== passwordForm.confirm_password) { ElMessage.error('两次密码不一致'); return }
  submitLoading.value = true
  try {
    const res = await axios.post('/admin/system/password', { old_password: passwordForm.old_password, new_password: passwordForm.new_password }, authHeaders())
    if (res.data.success) { ElMessage.success('密码已修改'); passwordForm.old_password = ''; passwordForm.new_password = ''; passwordForm.confirm_password = '' }
    else ElMessage.error(res.data.error || '修改失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '修改失败') }
  finally { submitLoading.value = false }
}
const toggleGuestRegister = async (val: number) => {
  try {
    const res = await axios.post('/admin/system/guest-register', { enabled: val }, authHeaders())
    if (res.data.success) ElMessage.success(val ? '已开启游客注册' : '已关闭游客注册')
    else { ElMessage.error(res.data.error || '操作失败'); systemConfig.guest_register_enabled = val ? 0 : 1 }
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '操作失败'); systemConfig.guest_register_enabled = val ? 0 : 1 }
}

const usernameForm = reactive({ new_username: '' })
const changeUsername = async () => {
  if (!usernameForm.new_username) { ElMessage.error('请输入新用户名'); return }
  submitLoading.value = true
  try {
    const res = await axios.post('/admin/system/username', { new_username: usernameForm.new_username }, authHeaders())
    if (res.data.success) { ElMessage.success('用户名已修改'); localStorage.setItem('username', usernameForm.new_username); username.value = usernameForm.new_username; usernameForm.new_username = '' }
    else ElMessage.error(res.data.error || '修改失败')
  } catch (e: any) { ElMessage.error(e.response?.data?.error || '修改失败') }
  finally { submitLoading.value = false }
}

// ==================== 通用 ====================
const handleTabClick = () => {
  if (activeTab.value === 'users') fetchUsers()
  else if (activeTab.value === 'logs') fetchLogs()
  else if (activeTab.value === 'cron') fetchCron()
  else if (activeTab.value === 'system') fetchSystemConfig()
}
const handleLogout = () => {
  localStorage.removeItem('token')
  localStorage.removeItem('username')
  localStorage.removeItem('admin_token')
  adminLoggedIn.value = false
  adminForm.username = ''
  adminForm.password = ''
}

onMounted(() => { if (isAdminLoggedIn.value) fetchUsers() })
</script>

<style scoped>
/* ===== pucoding.com pixel/retro design ===== */

/* 整体容器 */
.admin-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #f5f0e8;
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  color: #2c2c2c;
}

/* 顶部导航 */
.admin-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  height: 64px;
  background: #b4472f;
  color: white;
  position: relative;
  z-index: 10;
  border-bottom: 3px dashed #8c3625;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.logo-section {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo-icon {
  font-size: 28px;
}

.header-left h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.admin-badge {
  background: rgba(255, 255, 255, 0.2);
  padding: 4px 12px;
  border-radius: 2px;
  font-size: 12px;
  font-weight: 500;
  border: 2px dashed rgba(255, 255, 255, 0.4);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 20px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.15);
  padding: 8px 16px;
  border-radius: 2px;
  border: 2px dashed rgba(255, 255, 255, 0.3);
}

.user-avatar {
  font-size: 18px;
}

.username {
  font-size: 14px;
  font-weight: 500;
}

.logout-btn {
  color: white;
  border: 2px dashed rgba(255, 255, 255, 0.5);
  border-radius: 2px;
  padding: 8px 16px;
  background: transparent;
  box-shadow: 2px 2px 0 rgba(0, 0, 0, 0.15);
  transition: all 0.15s ease;
}

.logout-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  box-shadow: 2px 2px 0 rgba(0, 0, 0, 0.3);
}

.btn-icon {
  margin-right: 4px;
}

/* 主内容区 */
.admin-main {
  flex: 1;
  padding: 24px;
  background: #f5f0e8;
  overflow-y: auto;
}

/* Tab 样式 */
.admin-tabs {
  background: #faf5ed;
  border-radius: 2px;
  padding: 24px;
  border: 2px dashed #d4c8b0;
  box-shadow: 4px 4px 0 #d4c8b0;
}

.admin-tabs :deep(.el-tabs__header) {
  margin-bottom: 24px;
}

.admin-tabs :deep(.el-tabs__nav-wrap::after) {
  display: none;
}

.admin-tabs :deep(.el-tabs__item) {
  font-size: 15px;
  font-weight: 600;
  color: #666;
  transition: all 0.15s ease;
  padding: 0 20px;
  height: 40px;
  line-height: 40px;
}

.admin-tabs :deep(.el-tabs__item.is-active) {
  color: #b4472f;
}

.admin-tabs :deep(.el-tabs__active-bar) {
  background: #b4472f;
  height: 3px;
  border-radius: 2px;
}

/* Tab 头部 */
.tab-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  padding-bottom: 16px;
  border-bottom: 2px dashed #d4c8b0;
}

.tab-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #2c2c2c;
}

.tab-icon {
  font-size: 20px;
}

.tab-actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  border-radius: 2px;
  font-weight: 500;
  border: 2px dashed #b4472f;
  box-shadow: 2px 2px 0 #d4c8b0;
  transition: all 0.15s ease;
}

.action-btn:hover {
  box-shadow: 3px 3px 0 #b4a890;
}

/* 表格样式 */
.modern-table {
  border-radius: 2px;
  overflow: hidden;
  box-shadow: 2px 2px 0 #d4c8b0;
}

.modern-table :deep(.el-table__header-wrapper) {
  background: #f0ead8;
}

.modern-table :deep(.el-table__header th) {
  background: #f0ead8;
  color: #2c2c2c;
  font-weight: 600;
  font-size: 13px;
  border-bottom: 2px dashed #d4c8b0;
}

.modern-table :deep(.el-table__row) {
  transition: all 0.15s ease;
}

.modern-table :deep(.el-table__row:hover) {
  background: #f5f0e0;
}

.modern-table :deep(.el-table__row--striped) {
  background: #faf5ed;
}

.table-btn {
  margin: 1px;
  border-radius: 2px;
  font-size: 11px;
  padding: 4px 6px;
  border: 1px dashed #b4a890;
  box-shadow: 1px 1px 0 #d4c8b0;
}

/* Token 单元格 */
.token-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 2px;
  border: 2px dashed transparent;
  transition: all 0.15s ease;
}

.token-cell:hover {
  border-color: #b4472f;
  background: #faf5ed;
}

.token-tag {
  font-family: 'JetBrains Mono', 'Monaco', 'Menlo', monospace;
  font-size: 11px;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  border-radius: 2px;
}

.copy-hint {
  font-size: 12px;
  opacity: 0.5;
  flex-shrink: 0;
}

.token-cell:hover .copy-hint {
  opacity: 1;
}

.copy-btn {
  padding: 4px 8px;
  font-size: 12px;
  border-radius: 2px;
}

/* 系统设置区域 */
.system-section {
  max-width: 700px;
}

.system-section :deep(.el-card) {
  border-radius: 2px;
  border: 2px dashed #d4c8b0;
  background: #faf5ed;
  box-shadow: 3px 3px 0 #d4c8b0;
  transition: all 0.15s ease;
}

.system-section :deep(.el-card:hover) {
  box-shadow: 4px 4px 0 #b4a890;
}

.system-section :deep(.el-card__header) {
  background: #f0ead8;
  border-bottom: 2px dashed #d4c8b0;
  font-weight: 600;
  color: #2c2c2c;
  padding: 16px 20px;
}

.form-hint {
  font-size: 11px;
  color: #888;
  margin-top: 4px;
}

/* 响应式 */
@media (max-width: 768px) {
  .admin-header {
    padding: 0 16px;
    height: 56px;
  }

  .header-left h1 {
    font-size: 16px;
  }

  .admin-badge {
    display: none;
  }

  .user-info {
    padding: 6px 12px;
  }

  .admin-main {
    padding: 16px;
  }

  .admin-tabs {
    padding: 16px;
    border-radius: 2px;
  }

  .tab-header {
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }

  .tab-actions {
    width: 100%;
  }

  .action-btn {
    flex: 1;
  }

  .user-table :deep(.el-table__body-wrapper) {
    overflow-x: auto;
  }

  .token-tag {
    max-width: 80px;
    font-size: 10px;
  }

  .table-btn {
    padding: 4px 6px;
    font-size: 11px;
  }
}
/* ===== Admin Login Card (matches Login.vue style) ===== */
.login-container {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f0e8;
  z-index: 9999;
}

.bg-decoration {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
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

.btn-icon {
  font-size: 18px;
}

</style>
