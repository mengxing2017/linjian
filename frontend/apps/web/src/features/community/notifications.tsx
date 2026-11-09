import { Button } from 'antd'
import { PaginationControls } from '../../shared/pagination-controls'
import { formatDate } from '../../shared/format-date'
import { label } from '../../shared/label'
import { workspace, type Row } from './api'
import type { CommunityAction } from './actions'

export function Notifications({
  school: s,
  items,
  page,
  more,
  fetching,
  busy,
  onPage,
  run,
}: {
  school: string
  items: Row[]
  page: number
  more?: boolean
  fetching: boolean
  busy: boolean
  onPage: (page: number) => void
  run: CommunityAction
}) {
  return (
    <>
      {items.map((n) => (
        <p key={n.id}>
          {label(n.kind)} · {formatDate(n.created_at)}{' '}
          <Button
            disabled={busy || !!n.read_at}
            onClick={() =>
              run(() => workspace.mutate(s, `notifications/${n.id}/read`, undefined, 'PUT'))
            }
          >
            {n.read_at ? '已读' : '标记已读'}
          </Button>
        </p>
      ))}
      <PaginationControls page={page} more={more} busy={fetching} change={onPage} subject="消息" />
    </>
  )
}
