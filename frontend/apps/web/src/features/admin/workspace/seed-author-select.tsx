import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Select } from 'antd'
import { postSeed } from './api'
import { useSchoolPermission } from './permissions-context'
import { PaginationControls } from '../../../shared/pagination-controls'

type AuthorOption = { value: string; label: string }

export function SeedAuthorSelect({
  school,
  label,
  value,
  disabled,
  change,
}: {
  school: string
  label: string
  value?: string | null
  disabled: boolean
  change: (id: string | null) => void
}) {
  const can = useSchoolPermission()
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [selected, setSelected] = useState<AuthorOption>()
  const query = useQuery({
    queryKey: ['linjian', 'seed-authors', school, page, search.trim()],
    queryFn: ({ signal }) => postSeed.authors(school, page, search.trim(), signal),
    enabled: open && can('content.simulation.manage'),
  })
  return (
    <Select<AuthorOption>
      aria-label={label}
      style={{ minWidth: 240 }}
      disabled={disabled}
      labelInValue
      virtual={false}
      value={value ? selected : { value: '', label: '新建模拟作者' }}
      open={open}
      loading={query.isFetching}
      showSearch={{
        filterOption: false,
        searchValue: search,
        onSearch: (text) => {
          setSearch(text)
          setPage(1)
        },
      }}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) {
          setSearch('')
          setPage(1)
        }
      }}
      onChange={(option) => {
        setSelected(option)
        change(option.value || null)
      }}
      options={[
        { value: '', label: '新建模拟作者' },
        ...(query.data?.items ?? []).map((author) => ({
          value: author.id,
          label: author.nickname,
        })),
      ]}
      popupRender={(menu) => (
        <>
          {menu}
          {query.error && <Alert type="error" title={query.error.message} />}
          {query.isFetching && <p role="status">正在搜索模拟作者…</p>}
          {query.isSuccess && !query.isFetching && !query.data.items.length && (
            <p role="status">
              {search.trim()
                ? '未找到匹配的本校模拟作者，可修改昵称关键词或新建模拟作者。'
                : '本校暂无已创建的模拟作者；新作者在批次成功提交后才可搜索复用。'}
            </p>
          )}
          {(page > 1 || query.data?.page?.has_more) && (
            <div onMouseDown={(event) => event.preventDefault()}>
              <PaginationControls
                page={page}
                more={query.data?.page?.has_more}
                busy={disabled || query.isFetching}
                subject="模拟作者"
                change={setPage}
              />
            </div>
          )}
        </>
      )}
    />
  )
}
