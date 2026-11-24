<script setup lang="ts">
import { computed } from 'vue'
import type { SchoolDisplay } from './api'
import BrandImage from './BrandImage.vue'
import { defaultTheme, themeColor } from './branding'
import { formatRegion } from '@linjian/china-regions'
const props = defineProps<{ school?: SchoolDisplay; searchable?: boolean }>()
const emit = defineEmits<{ search: [] }>()
const regionLabel = computed(() =>
  props.school
    ? formatRegion({
        province: props.school.province,
        city: props.school.city,
        area: props.school.area,
      })
    : '',
)
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
      <view class="school-brand-copy">
        <view class="school-brand-title-row">
          <text class="school-brand-title">
            {{ school?.short_name || school?.name || '校园社区' }}
          </text>
          <button
            v-if="searchable"
            class="school-search"
            aria-label="搜索帖子"
            @tap.stop="emit('search')"
          >
            <text class="search-icon" />
            搜索
          </button>
        </view>
        <text>{{ regionLabel }}</text>
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
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.school-brand-copy {
  flex: 1;
  min-width: 0;
}
.school-brand-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16rpx;
}
.school-search {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8rpx;
  flex-shrink: 0;
  min-width: 44px;
  min-height: 44px;
  margin: 0;
  padding: 0 8px;
  font-size: 26rpx;
  color: inherit;
  background: transparent;
  line-height: 1.4;
}
.school-search::after {
  border: 0;
}
.search-icon {
  width: 22rpx;
  height: 22rpx;
  margin-right: 8rpx;
  border: 3rpx solid currentColor;
  border-radius: 50%;
  position: relative;
}
.search-icon::after {
  content: '';
  position: absolute;
  width: 12rpx;
  height: 3rpx;
  right: -10rpx;
  bottom: -5rpx;
  background: currentColor;
  transform: rotate(45deg);
}
</style>
