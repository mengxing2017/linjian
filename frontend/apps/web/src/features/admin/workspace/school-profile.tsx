import { MediaPicker } from '../operations/media-picker'
import { AuditModeFields } from './audit-modes'
import { useState, useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  ColorPicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tag,
} from 'antd'
import { workspace, type School, type SchoolInput, type SchoolFilters } from './api'
import { PaginationControls } from '../../../shared/pagination-controls'
import { actionColumn, RowActions } from '../../../shared/row-actions'
import { RegionCascader } from '../../../shared/region-cascader'
import { RegionField } from '../../../shared/region-field'
import { formatCoordinates } from '../../../shared/format-coordinates'
import { CoordinatesField } from '../../../shared/coordinates-field'
import {
  codesFromNames,
  formatRegion,
  namesFromCodes,
  type RegionValue,
} from '@linjian/china-regions'
const defaults = {
  name: '',
  short_name: '',
  province: '',
  city: '',
  area: '',
  latitude: null,
  longitude: null,
  logo_asset_id: null,
  header_bg_asset_id: null,
  theme_primary: '#6A5AF9',
  theme_secondary: '#8F6BFF',
  theme_text_color: '#FFFFFF',
}
type ThemeColorKey = 'theme_primary' | 'theme_secondary' | 'theme_text_color'
type ColorPreset = { label: string; colors: string[] }
const themeColorPresets: Record<ThemeColorKey, ColorPreset[]> = {
  theme_primary: [
    {
      label: '推荐品牌色',
      colors: ['#6A5AF9', '#1677FF', '#13C2C2', '#52C41A', '#FA8C16', '#EB2F96', '#722ED1', '#F5222D'],
    },
  ],
  theme_secondary: [
    {
      label: '推荐辅助色',
      colors: ['#8F6BFF', '#69B1FF', '#5CDBD3', '#95DE64', '#FFC069', '#FF85C0', '#B37FEB', '#FF7875'],
    },
  ],
  theme_text_color: [
    { label: '文字色', colors: ['#FFFFFF', '#F5F5F5', '#E5E7EB', '#1F2937', '#111827', '#000000'] },
  ],
}
function ThemeColorPicker({
  value,
  onChange,
  label,
  presets,
}: {
  value?: string
  onChange?: (value: string) => void
  label: string
  presets: ColorPreset[]
}) {
  return (
    <div role="group" aria-label={label}>
      <ColorPicker
        value={value}
        onChange={(color) => onChange?.(color.toHexString().toUpperCase())}
        showText
        disabledAlpha
        disabledFormat
        format="hex"
        presets={presets}
        placement="bottomLeft"
      />
    </div>
  )
}
type SchoolFormValues = Omit<SchoolInput, 'province' | 'city' | 'area'> & {
  region: RegionValue
}
export function SchoolImage({
  school,
  id,
  source,
  alt,
}: {
  school: string
  id?: string | null
  source?: string | null
  alt: string
}) {
  const [url, setUrl] = useState('')
  useEffect(() => {
    let live = true,
      object = ''
    setUrl('')
    if (id || source)
      void (
        id
          ? workspace.schoolProfiles.artwork(school, id)
          : workspace.schoolProfiles.branding(school, source!.endsWith('/logo') ? 'logo' : 'header')
      )
        .then((bytes) => {
          if (live) {
            object = URL.createObjectURL(new Blob([bytes]))
            setUrl(object)
          }
        })
        .catch(() => {})
    return () => {
      live = false
      if (object) URL.revokeObjectURL(object)
    }
  }, [school, id, source])
  return url ? (
    <img src={url} alt={alt} style={{ maxWidth: 200, maxHeight: 90 }} />
  ) : (
    <span>{alt}：默认</span>
  )
}
export function SchoolForm({
  school,
  platform = false,
  onSaved,
  onDirtyChange,
}: {
  school?: School
  platform?: boolean
  onSaved: (school?: School) => Promise<unknown>
  onDirtyChange?: (dirty: boolean) => void
}) {
  const [form] = Form.useForm<SchoolFormValues>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [conflict, setConflict] = useState(false),
    [done, setDone] = useState(false)
  const primary = Form.useWatch('theme_primary', form) || defaults.theme_primary,
    secondary = Form.useWatch('theme_secondary', form) || defaults.theme_secondary,
    text = Form.useWatch('theme_text_color', form) || defaults.theme_text_color
  const validColor = /^#[\da-fA-F]{3}(?:[\da-fA-F]{3})?$/
  const previewPrimary = validColor.test(primary) ? primary : defaults.theme_primary
  const previewSecondary = validColor.test(secondary) ? secondary : defaults.theme_secondary
  const previewText = validColor.test(text) ? text : defaults.theme_text_color
  const logo = Form.useWatch('logo_asset_id', form),
    header = Form.useWatch('header_bg_asset_id', form)
  async function save(v: SchoolFormValues) {
    if ((v.latitude == null) !== (v.longitude == null)) {
      setError('经纬度需同时填写或清空')
      return
    }
    setBusy(true)
    setError('')
    setDone(false)
    try {
      const { region, ...rest } = v
      const body = {
        ...defaults,
        ...rest,
        province: region?.province ?? '',
        city: region?.city ?? '',
        area: region?.area ?? '',
        revision: school?.revision ?? 0,
      }
      if (!platform) delete body.status
      const result = school
        ? await workspace.schoolProfiles.update(school.id, body)
        : await workspace.schoolProfiles.create(body)
      onDirtyChange?.(false)
      await onSaved(result)
      setDone(true)
    } catch (e) {
      setError((e as Error).message)
      if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  async function reload() {
    if (!school) return
    setBusy(true)
    try {
      const latest = await workspace.schoolProfiles.detail(school.id)
      onDirtyChange?.(false)
      await onSaved(latest)
      form.setFieldsValue({
        ...latest,
        region: { province: latest.province, city: latest.city, area: latest.area },
      })
      setConflict(false)
      setError('')
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Form
      form={form}
      layout="vertical"
      className="business-form form-grid"
      initialValues={{
        ...defaults,
        ...school,
        region: {
          province: school?.province ?? '',
          city: school?.city ?? '',
          area: school?.area ?? '',
        },
        status: school?.status ?? 'normal',
        config: school?.config ?? { post_audit_mode: 'manual', comment_audit_mode: 'manual' },
      }}
      onFinish={(v) => {
        if (v.status === 'hidden' && school?.status !== 'hidden') {
          Modal.confirm({
            title: '确认隐藏学校？',
            content: '成员将无法访问学校，已有来源将按原规则撤回。恢复学校不会批量重放历史内容。',
            okText: '确认隐藏',
            cancelText: '取消',
            onOk: () => save(v),
          })
        } else void save(v)
      }}
      onValuesChange={() => {
        setDone(false)
        onDirtyChange?.(true)
      }}
    >
      {error && <Alert type="error" title={error} />}
      {conflict && (
        <>
          <p>资料已被其他操作更新，本地输入已保留。重新加载会舍弃本地输入。</p>
          <Button onClick={() => void reload()}>重新加载资料</Button>
        </>
      )}
      {school && <h3 className="school-form-section">基本信息</h3>}
      <Form.Item
        label="名称"
        name="name"
        rules={[{ required: true, whitespace: true }, { max: 100 }]}
      >
        <Input maxLength={100} />
      </Form.Item>
      {(
        [
          ['short_name', '简称', 50],
        ] as const
      ).map(([name, label, max]) => (
        <Form.Item key={name} name={name} label={label} rules={[{ max }]}>
          <Input maxLength={max} />
        </Form.Item>
      ))}
      <Form.Item className="form-wide" label="省份 / 城市 / 区域" name="region">
        <RegionField />
      </Form.Item>
      <Form.Item className="form-wide" label="经纬度">
        <CoordinatesField />
      </Form.Item>
      <p>经纬度需同时填写或清空，最多六位小数；0 是有效坐标。</p>
      {platform && (
        <>
          <Form.Item name="status" label="状态">
            <Select
              options={[
                { value: 'normal', label: '正常' },
                { value: 'hidden', label: '隐藏' },
              ]}
            />
          </Form.Item>
          <p>隐藏后成员不可访问学校，已有来源将按原规则撤回；恢复不会批量重放历史内容。</p>
        </>
      )}
      {school ? (
        <>
          <h3 className="school-form-section">品牌与图片</h3>
          {(
            [
              ['logo_asset_id', '图标'],
              ['header_bg_asset_id', '头部背景图'],
            ] as const
          ).map(([key, label]) => (
            <section key={key}>
              <Form.Item name={key} label={label}>
                <MediaPicker school={school.id} value={form.getFieldValue(key) ?? null} label={label} onChange={id => form.setFieldValue(key, id)} onBusyChange={setBusy} />
              </Form.Item>
            </section>
          ))}
          <div className="form-wide form-grid form-grid--3">
            {(
              [
                ['theme_primary', '主题主色'],
                ['theme_secondary', '主题辅色'],
                ['theme_text_color', '主题文字色'],
              ] as const
            ).map(([key, label]) => (
              <Form.Item
                name={key}
                key={key}
                label={label}
                rules={[
                  { required: true },
                  { pattern: /^#(?:[\da-fA-F]{3}|[\da-fA-F]{6})$/, message: '请选择或输入 #RGB / #RRGGBB' },
                ]}
              >
                <ThemeColorPicker label={label} presets={themeColorPresets[key]} />
              </Form.Item>
            ))}
          </div>
          <div
            className="form-wide"
            style={{
              padding: 20,
              background: `linear-gradient(135deg,${previewPrimary},${previewSecondary})`,
              color: previewText,
            }}
          >
            <strong>品牌头部预览</strong>
            <SchoolImage school={school.id} id={logo} alt="学校图标" />
            <SchoolImage school={school.id} id={header} alt="头部背景图" />
            <div style={{ marginTop: 12, display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
              <span
                style={{
                  background: previewPrimary,
                  color: previewText,
                  padding: '4px 10px',
                  borderRadius: 4,
                }}
              >
                主色按钮
              </span>
              <span
                aria-label="主色到辅色渐变"
                style={{
                  flex: '1 1 120px',
                  height: 6,
                  borderRadius: 3,
                  background: `linear-gradient(90deg,${previewPrimary},${previewSecondary})`,
                }}
              />
            </div>
          </div>
          <h3 className="school-form-section">审核规则</h3>
          <AuditModeFields disabled={school.status !== 'normal'} />
        </>
      ) : (
        <p>图标、背景图与主题色可在创建后完善。</p>
      )}
      <div className="form-actions">
        <Button type="primary" htmlType="submit" loading={busy} disabled={conflict || busy}>
          {school ? (busy ? '正在保存…' : '保存资料') : busy ? '正在创建…' : '创建学校'}
        </Button>
      </div>
      {done && <Alert type="success" role="status" showIcon title="已保存" />}
    </Form>
  )
}
export function Schools({
  refresh,
  enter,
}: {
  schools: School[]
  loading: boolean
  refresh: () => Promise<void>
  enter: (id: string, view: string) => void
}) {
  const [filters, setFilters] = useState<SchoolFilters>({}),
    [creating, setCreating] = useState(false),
    [detail, setDetail] = useState<School>(),
    [editing, setEditing] = useState<School>(),
    [notice, setNotice] = useState<School>(),
    [highlight, setHighlight] = useState<string>(),
    [dirty, setDirty] = useState(false)
  const cache = useQueryClient()
  const q = useQuery({
    queryKey: ['linjian', 'school-management', filters],
    queryFn: ({ signal }) => workspace.schoolProfiles.platform(filters, signal),
  })
  useEffect(() => {
    if (!highlight) return
    const timer = window.setTimeout(() => setHighlight(undefined), 4000)
    return () => window.clearTimeout(timer)
  }, [highlight])
  useEffect(() => {
    if (!highlight || !q.data?.items.some((school) => school.id === highlight)) return
    document
      .querySelector(`[data-row-key="${highlight}"]`)
      ?.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' })
  }, [highlight, q.data])
  async function refreshSchools() {
    await cache.invalidateQueries({ queryKey: ['linjian', 'school-management'] })
    await refresh()
  }
  async function created(result?: School) {
    if (!result) return
    setCreating(false)
    setDirty(false)
    setNotice(result)
    setHighlight(result.id)
    await refreshSchools()
  }
  async function updated(result?: School) {
    if (!result) return
    setEditing(result)
    setDirty(false)
    await refreshSchools()
  }
  function openCreate() {
    setDirty(false)
    setEditing(undefined)
    setCreating(true)
  }
  function openEdit(school: School) {
    setDirty(false)
    setCreating(false)
    setEditing(school)
  }
  function closeEditor() {
    if (!dirty) {
      setCreating(false)
      setEditing(undefined)
      return
    }
    Modal.confirm({
      title: '有未保存的修改',
      content: '关闭后当前学校的未保存内容将丢失。',
      okText: '放弃修改',
      cancelText: '继续编辑',
      okButtonProps: { danger: true },
      onOk: () => {
        setCreating(false)
        setEditing(undefined)
        setDirty(false)
      },
    })
  }
  const change = (v: Partial<SchoolFilters>) => setFilters((f) => ({ ...f, ...v, page: 1 }))
  return (
    <Space orientation="vertical" className="full-width">
      <Button type="primary" onClick={openCreate}>
        新建
      </Button>
      {q.error && <Alert type="error" title={q.error.message} />}
      {notice && (
        <Alert
          type="success"
          showIcon
          role="status"
          closable
          onClose={() => setNotice(undefined)}
          title={`已创建「${notice.name}」。图标与背景图尚未配置。`}
          action={
            <Space>
              <Button
                size="small"
                onClick={() => {
                  openEdit(notice)
                  setNotice(undefined)
                }}
              >
                完善图标与背景
              </Button>
              <Button
                size="small"
                onClick={() =>
                  setFilters((current) => ({ ...current, search: notice.name, page: 1 }))
                }
              >
                在列表中定位
              </Button>
            </Space>
          }
        />
      )}
      <Space wrap>
        <Input
          aria-label="搜索名称、简称或区域"
          placeholder="名称 / 简称 / 区域"
          value={filters.search ?? ''}
          onChange={(e) => change({ search: e.target.value })}
        />
        <RegionCascader
          ariaLabel="筛选省份、城市或区域"
          placeholder="省 / 市 / 区"
          value={
            codesFromNames({
              province: filters.province ?? '',
              city: filters.city ?? '',
              area: filters.area ?? '',
            }) ?? []
          }
          onChange={(codes) => {
            const region = namesFromCodes(codes)
            change({
              province: region?.province ?? '',
              city: region?.city ?? '',
              area: region?.area ?? '',
            })
          }}
        />
        <Select
          aria-label="状态筛选"
          allowClear
          placeholder="全部状态"
          value={filters.status}
          options={[
            { value: 'normal', label: '正常' },
            { value: 'hidden', label: '隐藏' },
          ]}
          onChange={(status) => change({ status })}
        />
      </Space>
      <Table<School>
        className="linjian-data-table"
        tableLayout="fixed"
        rowKey="id"
        rowClassName={(s) => (s.id === highlight ? 'school-row-new' : '')}
        pagination={false}
        dataSource={q.data?.items ?? []}
        loading={q.isFetching}
        scroll={{ x: 1270 }}
        columns={[
          {
            title: '图标',
            width: 100,
            render: (_, s) => <SchoolImage school={s.id} id={s.logo_asset_id} alt="学校图标" />,
          },
          ...(['name', 'short_name'] as const).map((key, i) => ({
            title: ['名称', '简称'][i],
            dataIndex: key,
            width: [200, 140][i],
          })),
          {
            title: '地区',
            key: 'region',
            width: 240,
            ellipsis: true,
            render: (_, s) => formatRegion(s) || '—',
          },
          {
            title: '坐标',
            key: 'coordinates',
            width: 160,
            render: (_, s) => formatCoordinates(s.latitude, s.longitude),
          },
          { title: '状态', width: 100, render: (_, s) => (s.status === 'normal' ? '正常' : '隐藏') },
          {
            title: '资料',
            width: 110,
            render: (_, s) =>
              s.logo_asset_id && s.header_bg_asset_id ? (
                <Tag color="success">已完善</Tag>
              ) : (
                <Tag color="gold">待完善</Tag>
              ),
          },
          actionColumn<School>({
            width: 220,
            render: (s) => (
              <RowActions
                label={s.name}
                actions={[
                  { key: 'detail', label: '详情', onClick: () => setDetail(s) },
                  { key: 'edit', label: '编辑资料', onClick: () => openEdit(s) },
                  { key: 'settings', label: '设置', onClick: () => enter(s.id, 'settings') },
                  {
                    key: 'members',
                    label: '成员',
                    disabled: s.status === 'hidden',
                    onClick: () => enter(s.id, 'members'),
                  },
                ]}
              />
            ),
          }),
        ]}
      />
      <PaginationControls
        page={filters.page ?? 1}
        more={q.data?.page?.has_more}
        busy={q.isFetching}
        change={(page) => setFilters((v) => ({ ...v, page }))}
      />
      <Drawer
        open={creating || !!editing}
        size="large"
        title={editing ? `${editing.name} · 学校资料` : '新建学校'}
        onClose={closeEditor}
        destroyOnHidden
      >
        {creating ? (
          <SchoolForm key="new" platform onSaved={created} onDirtyChange={setDirty} />
        ) : editing ? (
          <SchoolForm
            key={editing.id}
            school={editing}
            platform
            onSaved={updated}
            onDirtyChange={setDirty}
          />
        ) : null}
      </Drawer>
      <Drawer
        open={!!detail}
        size="large"
        title={`${detail?.name ?? ''} · 详情`}
        onClose={() => setDetail(undefined)}
      >
        {detail && (
          <>
            <Descriptions
              column={1}
              items={[
                { key: 'name', label: '名称', children: detail.name || '未填写' },
                { key: 'short_name', label: '简称', children: detail.short_name || '未填写' },
                { key: 'region', label: '地区', children: formatRegion(detail) || '未填写' },
                {
                  key: 'coordinates',
                  label: '坐标',
                  children: formatCoordinates(detail.latitude, detail.longitude),
                },
                {
                  key: 'status',
                  label: '状态',
                  children: detail.status === 'normal' ? '正常' : '隐藏',
                },
                {
                  key: 'theme_primary',
                  label: '主题主色',
                  children: detail.theme_primary || '未填写',
                },
                {
                  key: 'theme_secondary',
                  label: '主题辅色',
                  children: detail.theme_secondary || '未填写',
                },
                {
                  key: 'theme_text_color',
                  label: '主题文字色',
                  children: detail.theme_text_color || '未填写',
                },
              ]}
            />
            <SchoolImage school={detail.id} id={detail.logo_asset_id} alt="学校图标" />
            <SchoolImage school={detail.id} id={detail.header_bg_asset_id} alt="头部背景图" />
          </>
        )}
      </Drawer>
    </Space>
  )
}
