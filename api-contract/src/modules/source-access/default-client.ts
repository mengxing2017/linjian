import { requestResource } from '../../http/browser-client'
import { createSourceAccessClient } from './client'
export const sourceAccess = createSourceAccessClient({ requestResource })
