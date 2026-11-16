import { Button, Flex, Space, Typography } from 'antd'
import { navigationSymbols } from './navigation'

type PaginationControlsProps = {
  page: number
  more?: boolean
  busy?: boolean
  subject?: string
  change: (page: number) => void
}

// These endpoints expose has_more without a total count; show only the known page number.
export function PaginationControls({ page, more, busy = false, subject = '', change }: PaginationControlsProps) {
  return <Flex className="pagination-controls" justify="space-between" align="center" wrap gap="small">
    <Typography.Text type="secondary">第 {page} 页</Typography.Text>
    <Space className="pagination-actions" wrap>
      <Button className="navigation-button" aria-label={`上一页${subject}`} disabled={busy || page <= 1} onClick={() => change(page - 1)}>{navigationSymbols.previous}</Button>
      <Button className="navigation-button" aria-label={`下一页${subject}`} disabled={busy || !more} onClick={() => change(page + 1)}>{navigationSymbols.next}</Button>
    </Space>
  </Flex>
}
