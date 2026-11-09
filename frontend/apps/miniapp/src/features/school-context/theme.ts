import { computed } from 'vue'
import { schoolContext } from './context'
import { defaultTheme, themeColor } from './branding'
export const schoolTheme = computed(() => {
  const school = schoolContext.schools.find((s) => s.id === schoolContext.id)
  return `--primary:${themeColor(school?.theme_primary, defaultTheme.theme_primary)};--secondary:${themeColor(school?.theme_secondary, defaultTheme.theme_secondary)};--school-theme-text:${themeColor(school?.theme_text_color, defaultTheme.theme_text_color)}`
})
