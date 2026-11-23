import { useEffect, useRef, useState } from 'react'
import { Alert, Button, Drawer, Space, Table, Upload } from 'antd'
import { workspace, type PostImportPreview } from './api'
import { createIdempotencyUuid } from '../../../shared/idempotency-key'
import { formatDate } from '../../../shared/format-date'
export function PostImport({
  school,
  close,
  saved,
  context,
  owner,
}: {
  school: string
  close: () => void
  saved: () => Promise<void>
  context?: string
  owner?: string
}) {
  const [job, setJob] = useState<PostImportPreview>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [actionError, setActionError] = useState(''),
    [uncertain, setUncertain] = useState(false)
  const key = useRef<string>(createIdempotencyUuid())
  const storageKey = owner ? `linjian:post-import:${owner}:${school}` : ''
  function remember(id: string, committing = false) {
    if (storageKey) {
      try {
        sessionStorage.setItem(storageKey, JSON.stringify({ id, key: key.current, committing }))
      } catch {
        /* Storage may be disabled. */
      }
    }
  }
  useEffect(() => {
    if (!storageKey) return
    let active = true
    try {
      const pending = JSON.parse(sessionStorage.getItem(storageKey) ?? 'null') as {
        id?: string
        key?: string
        committing?: boolean
      } | null
      if (!pending?.id || !pending.key) return
      key.current = pending.key
      setUncertain(!!pending.committing)
      setBusy(true)
      void workspace.postManagement
        .importDetail(school, pending.id)
        .then((result) => {
          if (active) {
            setJob(result)
            if (result.state !== 'preview') setUncertain(false)
          }
        })
        .catch((e: Error) => {
          if (active) setError(e.message)
        })
        .finally(() => {
          if (active) setBusy(false)
        })
    } catch {
      /* Ignore malformed local metadata; the server owns job contents. */
    }
    return () => {
      active = false
    }
  }, [school, storageKey])
  async function preview(file: File) {
    setBusy(true)
    setError('')
    setActionError('')
    setJob(undefined)
    setUncertain(false)
    key.current = createIdempotencyUuid()
    try {
      const result = await workspace.postManagement.preview(school, file)
      remember(result.id)
      setJob(result)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  async function commit() {
    if (!job) return
    setBusy(true)
    setActionError('')
    remember(job.id, true)
    try {
      const result = await workspace.postManagement.commit(school, job.id, {
        revision: job.revision,
        digest: job.digest,
        idempotency_key: key.current,
      })
      setJob(result)
      setUncertain(false)
      remember(result.id)
      await saved()
    } catch (e) {
      const pending = !(e && typeof e === 'object' && 'status' in e && Number(e.status) > 0)
      setActionError((e as Error).message)
      setUncertain(pending)
      remember(job.id, pending)
    } finally {
      setBusy(false)
    }
  }
  async function lookup() {
    if (!job) return
    setBusy(true)
    setActionError('')
    try {
      const latest = await workspace.postManagement.importDetail(school, job.id)
      setJob(latest)
      if (latest.state !== 'preview') {
        setUncertain(false)
        remember(latest.id)
      }
      if (latest.state === 'committed') await saved()
    } catch (e) {
      setActionError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Drawer open title="导入" size="large" onClose={close}>
      <p>{context}</p>
      <p>
        支持 UTF-8 CSV、XLS、XLSX，最多 200 条、5
        MiB、一个工作表。正文必填，标题选填且允许省略整列；标签名称和图集使用 JSON
        字符串数组。图片须为当前学校已有附件地址。每行作为当前账号在本校发布的正常、未置顶帖子。
      </p>
      <Space>
        <a
          download="帖子导入模板.csv"
          href={
            'data:text/csv;charset=utf-8,' +
            encodeURIComponent('\uFEFF标题,正文,标签名称,图集\n示例标题,示例正文,"[""校园""]",[]\n')
          }
        >
          下载 CSV 模板
        </a>
        <Upload
          accept=".csv,.xls,.xlsx"
          showUploadList={false}
          disabled={busy || uncertain}
          beforeUpload={(file) => {
            if (file.size > 5 * 1024 * 1024) setError('文件不得超过 5 MiB')
            else void preview(file)
            return false
          }}
        >
          <Button loading={busy}>选择文件并预览</Button>
        </Upload>
      </Space>
      {error && <Alert type="error" title={error} />}
      {job && (
        <>
          <p>
            预览有效期：{formatDate(job.expires_at)}；有效 {job.rows.length} 条，错误{' '}
            {job.errors.length} 条。
          </p>
          {job.state === 'expired' && <Alert type="warning" title="预览已过期，请重新选择文件" />}
          {job.state !== 'committed' && (
            <>
              {!!job.errors.length && (
                <Alert type="warning" title="存在错误，尚未创建任何帖子。请修正文件后重新预览。" />
              )}
              <Table
                className="linjian-data-table"
                tableLayout="fixed"
                rowKey="row"
                pagination={false}
                scroll={{ x: 520 }}
                dataSource={job.errors}
                columns={[
                  { title: '行号', width: 80, dataIndex: 'row' },
                  { title: '错误', width: 440, dataIndex: 'message' },
                ]}
              />
              <Table
                className="linjian-data-table"
                tableLayout="fixed"
                rowKey="row"
                scroll={{ x: 980 }}
                dataSource={job.rows}
                columns={[
                  { title: '行号', width: 80, dataIndex: 'row' },
                  { title: '内容概览', width: 220, render: (_, _row, index) => job.row_labels[index] },
                  { title: '正文', width: 360, render: (_, r) => r.post.content },
                  { title: '标签名称', width: 220, render: (_, r) => r.post.tags.join('、') },
                  { title: '图片数', width: 100, render: (_, r) => r.post.images.length },
                ]}
              />
            </>
          )}
        </>
      )}
      {(job || actionError) && (
        <div className="form-actions form-actions-sticky">
          {actionError && <Alert type="error" title={actionError} />}
          {job?.state === 'committed' && (
            <Alert type="success" role="status" title={`导入完成，共 ${job.result.length} 条帖子`} />
          )}
          <Space>
            {job?.state === 'committed' ? (
              <Button type="primary" onClick={close}>完成</Button>
            ) : (
              <>
                <Button
                  type="primary"
                  disabled={!job || job.state !== 'preview' || !!job.errors.length || !job.rows.length}
                  loading={busy}
                  onClick={() => void commit()}
                >
                  {uncertain ? '重试同一次导入' : '确认全部导入'}
                </Button>
                <Button disabled={busy} onClick={() => void lookup()}>
                  查询导入结果
                </Button>
              </>
            )}
          </Space>
        </div>
      )}
    </Drawer>
  )
}
