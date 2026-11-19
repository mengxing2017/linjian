import { PaginationControls } from '../../../shared/pagination-controls'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router-dom'
import { Alert, Button, Card, Drawer, Input, Popconfirm, Select, Space, Switch, Table, Tag, Upload } from 'antd'
import { browserClient, createBlockedWordsClient, requestMultipart, type BlockedWord, type BlockedWordImport } from './api'
import { actionColumn } from '../../../shared/row-actions'

export function BlockedWords({ school, schoolName, canEdit, platformAdmin = false }: { school?: string; schoolName?: string; canEdit: boolean; platformAdmin?: boolean }) {
  const [params] = useSearchParams()
  const platformParams = new URLSearchParams(params)
  platformParams.set('view', 'platform-blocked-words')
  platformParams.delete('subview')
  const api = createBlockedWordsClient(browserClient)
  const [global, setGlobal] = useState(false)
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState('all')
  const [page,setPage] = useState(1), [ordering,setOrdering] = useState(false)
  const [editing, setEditing] = useState<BlockedWord | null>()
  const [word, setWord] = useState(''), [remark, setRemark] = useState(''), [active, setActive] = useState(true)
  const [error, setError] = useState(''), [busy, setBusy] = useState(false)
  const [dragged, setDragged] = useState<string>()
  const [preview, setPreview] = useState<BlockedWordImport>()
  const q = useQuery({ queryKey: ['blocked-words', school, global, search, status, page, ordering], queryFn: () => api.list(school, global, search, status, page, ordering) })
  const editable = canEdit && !global
  const items = q.data?.items ?? []
  async function run(action: () => Promise<unknown>) { setBusy(true); setError(''); try { await action(); await q.refetch() } catch (e) { setError(e instanceof Error ? e.message : '操作失败') } finally { setBusy(false) } }
  function edit(item?: BlockedWord) { setEditing(item ?? null); setWord(item?.word ?? ''); setRemark(item?.remark ?? ''); setActive(item?.active ?? true) }
  function move(id: string, target: string) {
    const ids = items.map(v => v.id), from = ids.indexOf(id), to = ids.indexOf(target)
    if (from < 0 || to < 0 || from === to || !q.data) return
    ids.splice(from, 1); ids.splice(to, 0, id)
    void run(() => api.order(school, ids, q.data!.order_revision))
  }
  return <section>
    <Alert showIcon type="info" title={school
      ? '本校启用词条与平台启用词条共同生效，命中时不能发布。本校配置不能覆盖平台规则。'
      : '平台屏蔽词启用后对所有学校生效，命中时不能发布。'} />
    {school && global && platformAdmin && <Link to={{ search: `?${platformParams}` }}>前往平台屏蔽词管理</Link>}
    {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
    <Card title={school ? (global ? '平台屏蔽词（只读）' : `学校屏蔽词 · ${schoolName ?? school}`) : '平台屏蔽词管理'} extra={editable && <Space><Upload accept=".csv,.xlsx" showUploadList={false} beforeUpload={file=>{void run(async()=>setPreview(await requestMultipart<BlockedWordImport>(`${school?`/api/v1/schools/${encodeURIComponent(school)}`:'/api/v1/platform'}/blocked-word-imports`,file)));return false}}><Button>导入</Button></Upload><Button onClick={()=>{const url=URL.createObjectURL(new Blob(['\ufeff屏蔽词,备注,状态\r\n'],{type:'text/csv;charset=utf-8'}));const a=document.createElement('a');a.href=url;a.download='blocked-words-template.csv';a.click();URL.revokeObjectURL(url)}}>下载模板</Button><Button type="primary" onClick={() => edit()}>添加屏蔽词</Button></Space>}>
      <Space wrap>
        {school && <Select aria-label="范围" value={global ? 'global' : 'school'} onChange={v => {setGlobal(v === 'global');setPage(1);setOrdering(false)}} options={[{value:'school',label:'本校词条'},{value:'global',label:'平台词条（只读）'}]} />}
        <Input aria-label="查询屏蔽词或备注" placeholder="屏蔽词或备注" disabled={ordering} value={search} onChange={e => {setSearch(e.target.value);setPage(1)}} />
        <Select aria-label="状态筛选" disabled={ordering} value={status} onChange={v=>{setStatus(v);setPage(1)}} options={[{value:'all',label:'全部状态'},{value:'active',label:'启用'},{value:'inactive',label:'停用'}]} />
        {editable && <Button onClick={()=>{setSearch('');setStatus('all');setPage(1);setOrdering(!ordering)}}>{ordering?'退出排序':'排序模式'}</Button>}
      </Space>
      <Table<BlockedWord> className="linjian-data-table" tableLayout="fixed" rowKey="id" loading={q.isFetching || busy} dataSource={items} pagination={false} scroll={{ x: ordering ? 880 : 760 }} onRow={record => ({
        draggable: editable && ordering,
        onDragStart: () => setDragged(record.id), onDragOver: e => e.preventDefault(),
        onDrop: e => { e.preventDefault(); if (dragged) move(dragged,record.id); setDragged(undefined) },
      })} columns={[
        {title:'屏蔽词', width: 200,dataIndex:'word'}, {title:'备注', width: 320,dataIndex:'remark'},
        {title:'状态', width: 100,render:(_,r)=><Tag color={r.active?'success':'default'}>{r.active?'启用':'停用'}</Tag>},
        actionColumn<BlockedWord>({ width: ordering ? 260 : 140, render: (r) => editable && <Space size={4}><Button type="link" aria-label={`编辑 ${r.word}`} onClick={() => edit(r)}>编辑</Button><Popconfirm title="确定删除？" onConfirm={() => run(() => api.remove(school, r.id, r.revision))}><Button type="link" danger aria-label={`删除 ${r.word}`}>删除</Button></Popconfirm>
          {ordering && <><Button size="small" aria-label={`上移 ${r.word}`} disabled={items[0]?.id === r.id} onClick={() => move(r.id, items[items.indexOf(r) - 1].id)}>上移</Button><Button size="small" aria-label={`下移 ${r.word}`} disabled={items.at(-1)?.id === r.id} onClick={() => move(r.id, items[items.indexOf(r) + 1].id)}>下移</Button></>}</Space> }),
      ]} />
      {ordering ? <p>拖动行调整顺序，也可使用上移、下移按钮。</p> : <PaginationControls page={page} more={q.data?.has_more} busy={q.isFetching || busy} subject="屏蔽词" change={setPage}/>}
    </Card>
    <Drawer
      open={editing !== undefined}
      size="large"
      title={editing ? '编辑屏蔽词' : '添加屏蔽词'}
      onClose={() => setEditing(undefined)}
      footer={
        <Space>
          <Button onClick={() => setEditing(undefined)}>取消</Button>
          <Button
            type="primary"
            loading={busy}
            onClick={() =>
              void run(async () => {
                await api.save(
                  school,
                  { word, remark, active, revision: editing?.revision ?? 0 },
                  editing?.id,
                )
                setEditing(undefined)
              })
            }
          >
            保存
          </Button>
        </Space>
      }
    >
      <Space orientation="vertical" className="full-width"><label>屏蔽词<Input value={word} maxLength={100} onChange={e=>setWord(e.target.value)} /></label><label>备注<Input.TextArea value={remark} maxLength={255} onChange={e=>setRemark(e.target.value)} /></label><label>启用 <Switch checked={active} onChange={setActive}/></label></Space>
    </Drawer>
    <Drawer
      open={!!preview}
      size="large"
      title="导入预检"
      onClose={() => setPreview(undefined)}
      footer={
        <Space>
          <Button onClick={() => setPreview(undefined)}>取消</Button>
          <Button
            type="primary"
            loading={busy}
            disabled={!preview || preview.errors.length > 0}
            onClick={() =>
              void run(async () => {
                if (!preview) return
                await api.commitImport(school, preview.id, preview.digest)
                setPreview(undefined)
              })
            }
          >
            确认导入
          </Button>
        </Space>
      }
    >
      <p>有效词条 {preview?.rows.length ?? 0} 条；错误 {preview?.errors.length ?? 0} 条。全部通过后才能提交。</p>
      {preview?.errors.map((e,i)=><p key={i}>第 {e.row} 行：{e.message}</p>)}
    </Drawer>
  </section>
}
