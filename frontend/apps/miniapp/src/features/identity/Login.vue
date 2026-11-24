<script setup lang="ts">
import { ref } from 'vue'
import { restoreSession, session } from './session'
import { credentials } from '../../app/credentials'
import { form } from './onboarding-form'
const { progress, busy, error } = form
let entry = 0
let checkingSession: Promise<void> | undefined
const checking = ref(!!credentials.token)
function goHome() { return uni.reLaunch({ url: '/pages/index/index' }) }
function withTimeout(operation: Promise<void>, milliseconds: number) {
  return new Promise<void>((resolve, reject) => {
    const timer = globalThis.setTimeout(() => reject(new Error('登录状态检查超时')), milliseconds)
    operation.then(
      () => { globalThis.clearTimeout(timer); resolve() },
      failure => { globalThis.clearTimeout(timer); reject(failure) },
    )
  })
}
function checkSession() {
  if (!checkingSession)
    checkingSession = (async () => {
      if (!credentials.token) { checking.value = false; return }
      checking.value = true
      try { await withTimeout(restoreSession(), 15000) } catch { /* 失效或超时会话继续走登录/入驻流程 */ }
      finally { checking.value = false }
    })().finally(() => { checkingSession = undefined })
  return checkingSession
}
async function submit() {
  await form.begin()
  if (session.user?.bound_school_id) return
  if (progress.value) await uni.redirectTo({ url: '/pages/user/onboarding' })
}
defineExpose({
  show: async () => {
    const turn = ++entry
    await checkSession()
    if (turn !== entry) return
    if (session.user?.bound_school_id) {
      checking.value = true
      try { await goHome() } finally { checking.value = false }
      return
    }
    if (progress.value) await uni.redirectTo({ url: '/pages/user/onboarding' })
  },
})
</script>
<template>
  <view class="page login">
    <template v-if="checking || session.user?.bound_school_id">
      <text class="brand">Linjian</text>
      <text class="heading">{{ checking ? '正在检查登录状态' : '正在进入' }}</text>
      <text class="muted">{{ checking ? '请稍候…' : '正在返回首页…' }}</text>
      <button v-if="!checking" class="primary" @tap="goHome">返回首页</button>
    </template>
    <template v-else>
      <text class="brand">Linjian</text>
      <text class="heading">欢迎来到校园社区</text>
      <text class="muted">使用微信身份登录，首次登录需要确认协议并选择学校。</text>
      <text v-if="error" class="error">{{ error }}</text>
      <button class="primary" :loading="busy" :disabled="busy" @tap="submit">微信登录并进入</button>
      <text class="muted">首次登录会先验证微信身份，再确认协议与学校。</text>
    </template>
  </view>
</template>
