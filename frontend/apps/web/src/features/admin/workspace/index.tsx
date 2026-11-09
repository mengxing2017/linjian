import { SessionGate } from '../../identity'
import { WorkspaceShell } from './workspace-shell'

export { SessionGate } from '../../identity'
export { label } from '../../../shared/label'
export { stateLabel } from './state-label'

export function Workspace() {
  return (
    <SessionGate showSessionBar={false}>
      {(user, logout) => <WorkspaceShell user={user} logout={logout} />}
    </SessionGate>
  )
}
