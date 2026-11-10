<script setup lang="ts">
import { shallowRef } from 'vue'
import CustomerContacts from '../operations/CustomerContacts.vue'
const contacts = shallowRef<InstanceType<typeof CustomerContacts>>()
import Avatar from '../miniapp-users/Avatar.vue'
import { miniappUserApi, type MiniappUser } from '../miniapp-users/api'
import { session, restoreSession, logout } from '../identity/session'
import { schoolContext, loadSchools, scopeKey } from '../school-context/context'
import { usePage, navigate, confirm } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
const profile = shallowRef<MiniappUser>()
const { busy, error, run } = usePage(() => {
  profile.value = undefined
})
function load() {
  return run(async () => {
    await restoreSession()
    if (session.user) {
      await loadSchools()
      const scope = scopeKey()
      const value = await miniappUserApi.profile()
      if (scope === scopeKey()) { profile.value = value; await contacts.value?.show() }
    }
  })
}
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
defineExpose({ show: load, hide: () => contacts.value?.hide() })
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
    <CustomerContacts ref="contacts" />
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view v-if="session.user" class="card menu">
      <button @tap="navigate('/pages/user/userInfo')">个人信息</button>
      <button @tap="navigate('/pages/plugin/index')">学校</button>
      <button @tap="navigate('/pages/index/search?view=mine')">我的发布</button>
      <button @tap="navigate('/pages/index/search?view=collect')">我的收藏</button>
      <button @tap="navigate('/pages/user/attentions')">我的关注</button>
      <button @tap="navigate('/pages/user/message')">消息</button>
      <button @tap="navigate('/pages/index/category')">校园服务</button>
      <button @tap="navigate('/pages/user/legal')">学校说明</button>
      <button @tap="navigate('/pages/user/feedback')">投诉反馈</button>
      <button :disabled="busy" @tap="signout">退出登录</button>
    </view>
  </view>
</template>
