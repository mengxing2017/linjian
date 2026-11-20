import { Alert, Card } from 'antd'
import { useSchoolPermission } from './permissions-context'
import type { School } from './api'
import { AuditLog } from './audit-log'
import { Cases } from './cases'
import { ContentModeration } from './content-moderation'
import { Members } from './members'
import { Outbox } from './outbox'
import { SchoolForm } from './school-profile'
import { HomeMenu } from './home-menu'
import { SchoolResources } from './school-resources'
import { BlockedWords } from './blocked-words'

/** Authorized school page dispatch. Access checks and school selection live in WorkspaceShell. */
export function SchoolWorkspacePage({ view, school, platformAdmin, publisher, publisherId, refresh }: {
  publisherId?: string
  publisher?: string
  view: string
  school: School
  platformAdmin: boolean
  schoolAdmin: boolean
  refresh: () => Promise<void>
}) {
  const can = useSchoolPermission()
  switch (view) {
    case 'posts': return <ContentModeration publisherId={publisherId} publisher={publisher} platformAdmin={platformAdmin} school={school} canEditModes={can('school.review_settings.update')} refresh={refresh} />
    case 'home-menu': return <HomeMenu key={school.id} school={school.id} canEdit={can('school.home_menu.write')} />
    case 'cases': return <Cases school={school.id} refresh={refresh} />
    case 'resources': return <SchoolResources school={school} canEdit={can('resource.school.write')} refresh={refresh} />
    case 'blocked-words': return <BlockedWords key={school.id} school={school.id} schoolName={school.name} canEdit={can('school.blocked_word.write')} platformAdmin={platformAdmin} />
    case 'audit': return platformAdmin ? <AuditLog school={school.id} /> : <Alert type="warning" title="无权访问此页面" />
    case 'source-outbox': return <Outbox school={school.id} />
    case 'members': return <Members school={school} platformAdmin={platformAdmin} currentUserId={publisherId} refresh={refresh} />
    case 'settings': return <Card title={`${school.name} · 学校资料`}>
      <SchoolForm key={`${school.id}:${school.revision}`} school={school} platform={platformAdmin} onSaved={refresh} />
    </Card>
    default: return <Alert type="warning" title="页面不存在" />
  }
}
