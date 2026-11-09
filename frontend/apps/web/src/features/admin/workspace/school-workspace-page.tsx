import { Alert, Card } from 'antd'
import type { School } from './api'
import { AuditLog } from './audit-log'
import { Cases } from './cases'
import { ContentModeration } from './content-moderation'
import { Members } from './members'
import { Outbox } from './outbox'
import { SchoolForm } from './school-profile'
import { SchoolResources } from './school-resources'

/** Authorized school page dispatch. Access checks and school selection live in WorkspaceShell. */
export function SchoolWorkspacePage({ view, school, platformAdmin, schoolAdmin, refresh }: {
  view: string
  school: School
  platformAdmin: boolean
  schoolAdmin: boolean
  refresh: () => Promise<void>
}) {
  switch (view) {
    case 'posts': return <ContentModeration school={school} canEditModes={schoolAdmin} refresh={refresh} />
    case 'cases': return <Cases school={school.id} refresh={refresh} />
    case 'resources': return <SchoolResources school={school} canEdit={schoolAdmin} refresh={refresh} />
    case 'audit': return <AuditLog school={school.id} />
    case 'source-outbox': return <Outbox school={school.id} />
    case 'members': return <Members school={school} platformAdmin={platformAdmin} refresh={refresh} />
    case 'settings': return <Card title={`${school.name} · 学校设置`}>
      <SchoolForm key={`${school.id}:${school.revision}`} school={school} platform={platformAdmin} onSaved={refresh} />
    </Card>
    default: return <Alert type="warning" title="页面不存在" />
  }
}
