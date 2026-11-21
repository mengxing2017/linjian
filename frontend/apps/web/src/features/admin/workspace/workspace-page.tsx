import { SessionGate } from '../../identity'
import { WorkspaceShell } from './workspace-shell'
export function Workspace() {
  return (
    <SessionGate showSessionBar={false}>
      {(user, logout) => <WorkspaceShell user={user} logout={logout} />}
    </SessionGate>
  )
}
