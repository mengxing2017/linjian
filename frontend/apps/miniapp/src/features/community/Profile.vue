<script setup lang="ts">
import { onUnmounted, shallowRef } from 'vue'
import CustomerContacts from '../operations/CustomerContacts.vue'
import MenuItem from '../../shared/MenuItem.vue'
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
  <view class="page compact-page profile-page">
    <view class="card">
      <view class="profile-summary">
        <Avatar :source="profile?.avatar_url" />
        <view class="profile-copy">
          <text class="heading">
            {{ profile?.nickname || session.user?.display_name || '校园成员' }}
          </text>
          <text class="muted">
            {{ schoolContext.schools.find((s) => s.id === schoolContext.id)?.name || '尚未绑定学校' }}
          </text>
        </view>
      </view>
      <button v-if="!session.user" class="primary" @tap="navigate('/pages/user/login')">
        微信登录
      </button>
    </view>
    <CustomerContacts ref="contacts" @retry-prepare="page.retryPrepare" />
    <PageStatus :busy="busy" :error="error" @retry="page.reloadProfile" />
    <text v-if="signoutError" class="error">{{ signoutError }}</text>
    <view v-if="session.user" class="card menu profile-menu">
      <MenuItem title="个人信息" icon="person" @activate="navigate('/pages/user/userInfo')" />
      <MenuItem title="学校" icon="school" @activate="navigate('/pages/plugin/index')" />
      <MenuItem v-if="session.user.bound_school_id" title="换校申请与审核结果" icon="application" @activate="navigate('/pages/user/schoolChange')" />
      <MenuItem title="我的发布" icon="posts" @activate="navigate('/pages/index/search?view=mine')" />
      <MenuItem title="我的收藏" icon="collection" @activate="navigate('/pages/index/search?view=collect')" />
      <MenuItem title="我的关注" icon="attention" @activate="navigate('/pages/user/attentions')" />
      <MenuItem title="消息" icon="inbox" @activate="navigate('/pages/user/message')" />
      <MenuItem title="校园服务" icon="services" @activate="navigate('/pages/index/category')" />
      <MenuItem title="学校说明" icon="info" @activate="navigate('/pages/user/legal')" />
      <MenuItem title="投诉反馈" icon="posts" @activate="navigate('/pages/user/feedback')" />
      <MenuItem title="退出登录" :arrow="false" danger :disabled="signingOut" @activate="signout" />
    </view>
  </view>
</template>
<style scoped>
.profile-summary { display: flex; align-items: center; gap: 20rpx; }
.profile-copy { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.profile-copy .heading { font-size: 32rpx; margin: 0; }
.profile-copy .muted { margin: 4rpx 0 0; line-height: 1.5; }
.profile-menu :deep(.menu-item) { min-height: 44px; padding: 8rpx 4rpx; gap: 16rpx; }
.profile-menu :deep(.menu-item-icon) { width: 52rpx; height: 52rpx; border-radius: 14rpx; }
</style>
