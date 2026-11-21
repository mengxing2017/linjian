export const part = (s: string) => encodeURIComponent(s)
export const scope = (school: string, path: string) => `/api/v1/schools/${part(school)}/${path}`
