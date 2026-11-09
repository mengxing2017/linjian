<script setup lang="ts">
import { ref, shallowRef } from 'vue'
import { restoreSession, refreshUser, session } from '../identity/session'
import { miniappUserApi, type MiniappUser } from '../miniapp-users/api'
import { uploadAvatar } from '../miniapp-users/avatar-files'
import Avatar from '../miniapp-users/Avatar.vue'
import { usePage } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import { scopeKey } from '../school-context/context'
const profile = shallowRef<MiniappUser>(),
  name = ref(''),
  mobile = ref(''),
  asset = ref<string | null>(null),
  avatar = ref<string | null>(null)
const { busy, error, run } = usePage(() => {
  profile.value = undefined
  name.value = ''
  mobile.value = ''
  asset.value = null
  avatar.value = null
})
async function read() {
  await restoreSession()
  if (!session.user) return
  const scope = scopeKey()
  const value = await miniappUserApi.profile()
  if (scope !== scopeKey()) return
  profile.value = value
  name.value = value.nickname
  mobile.value = value.mobile ?? ''
  asset.value = value.avatar_asset_id
  avatar.value = value.avatar_url
}
function load() {
  return run(() => read())
}
function choose(event: { detail: { avatarUrl: string } }) {
  void run(async (current) => {
    const result = await uploadAvatar(event.detail.avatarUrl)
    if (current()) {
      asset.value = result.id
      avatar.value = result.url
    }
  })
}
function save() {
  void run(async () => {
    if (!profile.value) return
    await miniappUserApi.saveProfile(name.value, asset.value, profile.value.revision)
    await refreshUser()
    await read()
    uni.showToast({ title: '已更新' })
  })
}
function registerMobile() {
  void run(async () => {
    if (!profile.value) return
    await miniappUserApi.registerMobile(mobile.value, profile.value.revision)
    await read()
    uni.showToast({ title: '手机号已登记' })
  })
}
defineExpose({ show: load })
</script>
<template>
  <view class="page">
    <text class="heading">个人信息</text>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view v-if="profile" class="card">
      <Avatar :source="avatar" />
      <button open-type="chooseAvatar" :disabled="busy" @chooseavatar="choose">选择头像</button>
      <text>昵称</text>
      <input v-model="name" type="nickname" maxlength="100" class="input" placeholder="昵称" />
      <button class="primary" :disabled="busy || !name.trim()" @tap="save">保存头像昵称</button>
      <text>手机号</text>
      <input
        v-model="mobile"
        type="number"
        maxlength="11"
        class="input"
        placeholder="未填写"
        :disabled="!!profile.mobile"
      />
      <text class="muted">联系手机号为本人填写，未作验证；登记后需联系平台管理员纠正。</text>
      <button v-if="!profile.mobile" :disabled="busy || !mobile" @tap="registerMobile">
        登记手机号
      </button>
      <text>所属学校：{{ profile.school_name || '未绑定学校' }}</text>
      <text>发帖数：{{ profile.post_count }}</text>
      <text>创建时间：{{ new Date(profile.created_at).toLocaleString() }}</text>
      <text>状态：{{ profile.status === 'normal' ? '正常' : '封禁' }}</text>
    </view>
  </view>
</template>
