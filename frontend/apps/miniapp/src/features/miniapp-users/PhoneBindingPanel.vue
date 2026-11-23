<script setup lang="ts">
import { onUnmounted } from 'vue'
import { miniappUserApi } from './api'
import { createPhoneBinding } from './phone-binding'
import { scopeKey, onScopeChange } from '../school-context/context'
import { uuid } from '../operations/policy'
import { confirm, handlePageError } from '../../shared/use-page'

defineProps<{ manage?: boolean }>()
const phone = createPhoneBinding({ api: miniappUserApi, key: scopeKey, uuid, confirm, failure: handlePageError, notify: title => uni.showToast({ title, icon: 'none' }) })
const { binding, visible, busy, error } = phone
const stop = onScopeChange(phone.reset)
onUnmounted(() => { stop(); phone.dispose() })
defineExpose({ ensure: phone.ensure, open: phone.open, close: phone.close })
</script>
<template>
  <view v-if="manage" class="card">
    <text>微信手机号实名认证</text>
    <text class="muted">{{ binding?.status === 'verified' ? `已验证 ${binding.masked_phone}` : '发布前需验证，暂不验证仍可登录浏览。' }}</text>
    <button :disabled="busy" @tap="phone.open">{{ binding?.status === 'verified' ? '管理或更换手机号' : '验证或查看绑定状态' }}</button>
  </view>
  <view v-if="visible" class="phone-backdrop">
    <view class="card phone-panel">
      <text class="title">{{ binding?.status === 'verified' ? '更换或撤回手机号' : '发布前，验证一下手机号' }}</text>
      <text class="muted">发布、评论和点赞前需通过微信验证手机号。号码不会展示给其他用户，不用于营销。暂不验证仍可登录浏览。</text>
      <text v-if="binding?.status === 'verified'">当前号码：{{ binding.masked_phone }}</text>
      <text v-if="error" class="error">{{ error }}</text>
      <button open-type="getPhoneNumber" :disabled="busy || !binding" :loading="busy" @getphonenumber="phone.authorize">{{ binding?.status === 'verified' ? '微信授权更换手机号' : '微信授权验证手机号' }}</button>
      <button v-if="binding?.status === 'verified'" :disabled="busy" @tap="phone.revoke">撤回绑定并删除当前号码</button>
      <button v-if="!binding" :disabled="busy" @tap="phone.open">重新读取状态</button>
      <button @tap="phone.close">暂不验证 / 关闭</button>
      <navigator url="/pages/user/phonePrivacy">查看手机号隐私规则</navigator>
      <text class="muted">如微信版本或账号不支持验证，请更新微信或稍后重试。验证后不会自动提交内容，请返回确认。</text>
    </view>
  </view>
</template>
<style scoped>
.phone-backdrop { position: fixed; inset: 0; z-index: 1000; background: rgba(0, 0, 0, .45); display: flex; align-items: flex-end; }
.phone-panel { width: 100%; max-height: 85vh; overflow-y: auto; margin: 0; padding-bottom: calc(32rpx + env(safe-area-inset-bottom)); }
</style>
