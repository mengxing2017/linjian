import type { PaginationProps } from 'antd'

export const navigationSymbols = { previous: '<', next: '>' } as const

// Keep rc-pagination's parent click/keyboard handlers and disabled propagation.
export const navigationPagination: Pick<PaginationProps, 'itemRender' | 'showTitle'> = {
  showTitle: false,
  itemRender: (_, type, original) => {
    if (type === 'page') return original
    const previous = type === 'prev' || type === 'jump-prev'
    const label = type === 'prev' ? '上一页' : type === 'next' ? '下一页'
      : previous ? '向前跳页' : '向后跳页'
    return <button type="button" className="ant-pagination-item-link navigation-button" tabIndex={-1} aria-label={label}>
      {previous ? navigationSymbols.previous : navigationSymbols.next}
    </button>
  },
}
