<script setup lang="ts">
import { onUnmounted, shallowRef } from 'vue'
import CustomerContacts from '../operations/CustomerContacts.vue'
import Avatar from '../miniapp-users/Avatar.vue'
import { miniappUserApi, type MiniappUser } from '../miniapp-users/api'
import { session, restoreSession, logout } from '../identity/session'
import { schoolContext, loadSchools, scopeKey, onScopeChange } from '../school-context/context'
import { credentials } from '../../app/credentials'
import { usePage, navigate, confirm, handlePageError } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import { createContactsPage } from './use-contacts-page'
const contacts = shallowRef<InstanceType<typeof CustomerContacts>>()
const profile = shallowRef<MiniappUser>()
const page = createContactsPage({
  contacts: () => contacts.value,
  restore: restoreSession,
  loggedIn: () => !!session.user,
  loadSchools,
  school: () => schoolContext.id,
  revision: () => credentials.revision,
  key: scopeKey,
  profile: async current => {
    const value = await miniappUserApi.profile()
    if (current()) profile.value = value
  },
  failure: handlePageError,
})
const { busy, error } = page
const { busy: signingOut, error: signoutError, run } = usePage()
const stop = onScopeChange(() => { profile.value = undefined; page.resetProfile() })
onUnmounted(() => { stop(); page.dispose() })
function signout() {
  void run(async () => {
    if (!(await confirm('确认退出登录？'))) return
    try {
      await logout()
    } catch {
      uni.showToast({ title: '本机已退出，服务端注销未确认', icon: 'none' })
    }
    uni.reLaunch({ url: '/pages/user/login' })
  })
}
defineExpose({ show: page.show, hide: page.hide })
</script>
<template>
  <view class="page">
    <view class="card">
      <Avatar :source="profile?.avatar_url" />
      <text class="heading">
        {{ profile?.nickname || session.user?.display_name || '校园成员' }}
      </text>
      <text class="muted">
        {{ schoolContext.schools.find((s) => s.id === schoolContext.id)?.name || '尚未绑定学校' }}
      </text>
      <button v-if="!session.user" class="primary" @tap="navigate('/pages/user/login')">
        微信登录
      </button>
    </view>
    <CustomerContacts ref="contacts" @retry-prepare="page.retryPrepare" />
    <PageStatus :busy="busy" :error="error" @retry="page.reloadProfile" />
    <text v-if="signoutError" class="error">{{ signoutError }}</text>
    <view v-if="session.user" class="card menu">
      <button @tap="navigate('/pages/user/userInfo')">个人信息</button>
      <button @tap="navigate('/pages/plugin/index')">学校</button>
      <button v-if="session.user.bound_school_id" @tap="navigate('/pages/user/schoolChange')">换校申请与审核结果</button>
      <button @tap="navigate('/pages/index/search?view=mine')">我的发布</button>
      <button @tap="navigate('/pages/index/search?view=collect')">我的收藏</button>
      <button @tap="navigate('/pages/user/attentions')">我的关注</button>
      <button @tap="navigate('/pages/user/message')">消息</button>
      <button @tap="navigate('/pages/index/category')">校园服务</button>
      <button @tap="navigate('/pages/user/legal')">学校说明</button>
      <button @tap="navigate('/pages/user/feedback')">投诉反馈</button>
      <button :disabled="signingOut" @tap="signout">退出登录</button>
    </view>
  </view>
</template>
