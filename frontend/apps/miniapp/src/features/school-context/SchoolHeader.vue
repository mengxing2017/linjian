<script setup lang="ts">
import { computed } from 'vue'
import type { SchoolDisplay } from './api'
import BrandImage from './BrandImage.vue'
import { defaultTheme, themeColor } from './branding'
const props = defineProps<{ school?: SchoolDisplay }>()
const style = computed(() => {
  const p = themeColor(props.school?.theme_primary, defaultTheme.theme_primary),
    s = themeColor(props.school?.theme_secondary, defaultTheme.theme_secondary),
    t = themeColor(props.school?.theme_text_color, defaultTheme.theme_text_color)
  return {
    '--school-theme-primary': p,
    '--school-theme-secondary': s,
    '--school-theme-text': t,
    background: `linear-gradient(135deg,${p},${s})`,
    color: t,
  }
})
</script>
<template>
  <view class="school-brand" :style="style">
    <BrandImage
      v-if="school"
      :school="school.id"
      purpose="header"
      :revision="school.revision"
      :enabled="!!school.header_bg_image_url"
    />
    <view class="school-brand-content">
      <BrandImage
        v-if="school"
        :school="school.id"
        purpose="logo"
        :revision="school.revision"
        :enabled="!!school.logo_url"
      />
      <view>
        <text class="school-brand-title">
          {{ school?.short_name || school?.name || '校园社区' }}
        </text>
        <text>{{ [school?.province, school?.city, school?.area].filter(Boolean).join(' ') }}</text>
      </view>
    </view>
  </view>
</template>
<style scoped>
.school-brand {
  position: relative;
  overflow: hidden;
  border-radius: 24rpx;
  padding: 32rpx;
  margin-bottom: 24rpx;
}
.school-brand-content {
  position: relative;
  display: flex;
  gap: 24rpx;
  align-items: center;
}
.school-brand-title {
  display: block;
  font-size: 40rpx;
  font-weight: 700;
  margin-bottom: 8rpx;
}
</style>
