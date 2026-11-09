export const schoolRoleLabel = (role?: string) =>
  ({ member: '普通成员', operator: '运营', admin: '学校管理员', revoked: '已撤销学校授权' })[
    role ?? ''
  ] ?? '无学校授权'
