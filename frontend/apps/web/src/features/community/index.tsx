import { SessionGate } from '../identity'
import { CommunityShell } from './community-shell'
import { CommunityContent } from './community-content'

export default function Community() {
  return (
    <CommunityShell>
      <SessionGate>{(user) => <CommunityContent user={user} />}</SessionGate>
    </CommunityShell>
  )
}
