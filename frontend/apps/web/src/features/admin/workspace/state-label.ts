import type { Json } from './api'
import { label } from '../../../shared/label'

export const states: Record<string, string> = {
  member: '普通成员',
  operator: '运营',
  admin: '学校管理员',
  revoked: '已撤销学校授权',
  pending: '待审核',
  approved: '已通过',
  rejected: '已拒绝',
  visible: '展示中',
  hidden: '已隐藏',
  deleted: '已删除',
  prepared: '等待客户端结果',
  client_reported_success: '客户端报告成功',
  client_reported_failure: '客户端报告失败',
  unknown: '结果不确定',
  cancelled: '已取消',
  operator_confirmed: '人工确认已发送',
  operator_rejected: '人工确认未发送',
  open: '待处理',
  resolved: '已处理',
  dismissed: '已驳回',
}
export const stateLabel = (v: Json | undefined) => states[label(v)] ?? label(v)
