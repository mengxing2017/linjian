import type { Post } from './api'

export function postVisibilityLabel(visibility: Post['visibility']): string {
  return {visible: '正常', hidden: '隐藏', offline: '系统下架', deleted: '已删除'}[visibility]
}
