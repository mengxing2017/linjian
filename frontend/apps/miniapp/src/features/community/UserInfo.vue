<script setup lang="ts">
import { onUnmounted, ref } from 'vue'
import { restoreSession, refreshUser, session } from '../identity/session'
import { miniappUserApi } from '../miniapp-users/api'
import { uploadAvatar } from '../miniapp-users/avatar-files'
import Avatar from '../miniapp-users/Avatar.vue'
import PhoneBindingPanel from '../miniapp-users/PhoneBindingPanel.vue'
import { confirm, handlePageError } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import { onScopeChange, scopeKey, loadSchools, schoolContext } from '../school-context/context'
import { credentials } from '../../app/credentials'
import { createUserInfo } from './use-user-info'
import { createContactsPage } from './use-contacts-page'
import CustomerContacts from '../operations/CustomerContacts.vue'
import LegalLinks from '../legal-documents/LegalLinks.vue'
const policies = ref<InstanceType<typeof LegalLinks>>()
const phone = ref<InstanceType<typeof PhoneBindingPanel>>()
const contacts = ref<InstanceType<typeof CustomerContacts>>()

const info = createUserInfo({
  api: miniappUserApi,
  restore: async () => { await restoreSession(); return !!session.user },
  refreshUser,
  upload: uploadAvatar,
  key: scopeKey,
  failure: handlePageError,
  confirm,
  notify: (title) => uni.showToast({ title }),
})
const { profile, name, avatar, uploadResult, busy, error, conflict, refreshWarning, writeUncertain, length, canSave } = info
const page = createContactsPage({
  contacts: () => contacts.value,
  restore: restoreSession,
  loggedIn: () => !!session.user,
  loadSchools,
  school: () => schoolContext.id,
  revision: () => credentials.revision,
  key: scopeKey,
  profile: () => info.show(),
  failure: handlePageError,
})
const { busy: preparing, error: preparationError } = page
const stop = onScopeChange(() => { info.reset(); page.resetProfile() })
onUnmounted(() => { stop(); page.dispose(); info.dispose() })
function choose(event: { detail: { avatarUrl?: string } }) {
  void info.choose(event.detail.avatarUrl ?? '')
}
defineExpose({ show: () => { void policies.value?.show(); return page.show() }, hide: () => { page.hide(); info.hide(); phone.value?.close(); policies.value?.hide() }, refresh: () => { void policies.value?.show(); return info.reload() } })
</script>
<template>
  <view class="page">
    <text class="heading">个人信息</text>
    <PageStatus v-if="!profile" :busy="busy || preparing" :error="error || preparationError" @retry="page.reloadProfile" />
    <view v-if="profile" class="card">
      <Avatar :source="avatar" />
      <button open-type="chooseAvatar" :disabled="busy" @chooseavatar="choose">上传头像</button>
      <text class="muted">可选择自己的图片，上传后点击保存生效。</text>
      <text v-if="uploadResult">{{ uploadResult }}</text>
      <text>昵称</text>
      <input v-model="name" type="nickname" :maxlength="-1" :disabled="busy" class="input" placeholder="昵称" />
      <text class="muted">{{ length }}/100 个字符，昵称允许重名。</text>
      <text v-if="error" class="error">{{ error }}</text>
      <button v-if="conflict || writeUncertain" :disabled="busy" @tap="info.reload">重新读取资料</button>
      <button class="primary" :disabled="!canSave" @tap="info.save">保存头像昵称</button>
      <text v-if="refreshWarning" class="error">{{ refreshWarning }}</text>
      <button v-if="refreshWarning" :disabled="busy" @tap="info.retryRefresh">重试刷新显示信息</button>
      <PhoneBindingPanel ref="phone" manage />
      <text>所属学校：{{ profile.school_name || '未绑定学校' }}</text>
      <text>发帖数：{{ profile.post_count }}</text>
      <text>创建时间：{{ new Date(profile.created_at).toLocaleString() }}</text>
      <text>状态：{{ profile.status === 'normal' ? '正常' : '封禁' }}</text>
    </view>
    <CustomerContacts ref="contacts" @retry-prepare="page.retryPrepare" />
    <LegalLinks ref="policies" />
  </view>
</template>
