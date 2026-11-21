import { requestCollection, requestResource } from '../../http/browser-client'
import { createProductClient } from './client'
export const product = createProductClient({ requestResource, requestCollection })
