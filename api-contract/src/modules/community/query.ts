export function pageQuery(page = 1) {
  return `page[number]=${page}&page[size]=20`
}
