<script setup lang="ts">
import { computed } from 'vue'
import type { CommunityPost } from './api'
import { postFeedCopy, postFeedStatus } from './post-feed-card'
import AppIcon from '../../shared/AppIcon.vue'
import PrivateImage from '../media/PrivateImage.vue'
const props = defineProps<{ post: CommunityPost; school: string; mine?: boolean }>()
const emit = defineEmits<{ open: [post: CommunityPost] }>()
const copy = computed(() => postFeedCopy(props.post))
</script>
<template>
  <view class="feed-card" role="link" aria-label="查看帖子详情" @tap="emit('open', post)">
    <view class="feed-meta">
      <text class="feed-author">{{ post.author_name || '校园成员' }}</text>
      <text class="feed-time">{{ post.published_at_text }}</text>
    </view>
    <view class="feed-content">
      <view class="feed-copy">
        <text v-if="copy.title" class="feed-title">{{ copy.title }}</text>
        <text class="feed-summary">{{ copy.summary }}</text>
      </view>
      <view v-if="post.images[0]" class="feed-media">
        <PrivateImage :school="school" :source="post.images[0]" variant="feed" />
      </view>
    </view>
    <view class="feed-counts">
      <text>
        <AppIcon name="like" :size="26" />
        {{ post.like_count }}
      </text>
      <text>
        <AppIcon name="comment" :size="26" />
        {{ post.comment_count }}
      </text>
    </view>
    <text v-if="mine" class="feed-status">{{ postFeedStatus(post) }}</text>
  </view>
</template>
<style scoped>
.feed-card {
  width: 100%;
  box-sizing: border-box;
  padding: var(--linjian-space-card);
  background: #fff;
  border-radius: var(--linjian-radius-card);
}
.feed-meta {
  display: flex;
  align-items: baseline;
  gap: 12rpx;
  margin-bottom: 8rpx;
  color: #6b7280;
  font-size: var(--linjian-font-meta);
  line-height: 1.4;
}
.feed-author {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.feed-time {
  max-width: 65%;
  overflow-wrap: anywhere;
  font-size: var(--linjian-font-caption);
}
.feed-content {
  display: flex;
  align-items: flex-start;
  gap: 16rpx;
}
.feed-copy {
  flex: 1;
  min-width: 0;
}
.feed-title,
.feed-summary {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  overflow: hidden;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
.feed-title {
  -webkit-line-clamp: 2;
  margin-bottom: 8rpx;
  font-size: var(--linjian-font-title);
  line-height: 1.4;
  font-weight: 600;
}
.feed-summary {
  -webkit-line-clamp: 3;
  font-size: var(--linjian-font-body);
  line-height: var(--linjian-line-body);
}
.feed-media {
  flex: 0 0 152rpx;
  width: 152rpx;
}
.feed-counts {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 20rpx;
  margin-top: 8rpx;
  color: #6b7280;
  font-size: var(--linjian-font-caption);
  line-height: 1.4;
}
.feed-status {
  display: block;
  margin-top: 8rpx;
  font-size: var(--linjian-font-meta);
  color: #6b7280;
  line-height: 1.5;
  overflow-wrap: anywhere;
}
</style>
