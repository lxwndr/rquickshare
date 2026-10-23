import { ref, watch } from 'vue'

export type ThemePreference = 'system' | 'light' | 'dark'

const storageKey = 'rquickshare-theme'
const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

function readPreference(): ThemePreference {
  const saved = localStorage.getItem(storageKey)
  return saved === 'light' || saved === 'dark' || saved === 'system' ? saved : 'system'
}

export const themePreference = ref<ThemePreference>(readPreference())

function applyTheme(): void {
  const isDark = themePreference.value === 'dark'
    || (themePreference.value === 'system' && mediaQuery.matches)

  document.documentElement.classList.toggle('dark', isDark)
  document.documentElement.style.colorScheme = isDark ? 'dark' : 'light'
}

export function setThemePreference(preference: ThemePreference): void {
  themePreference.value = preference
  localStorage.setItem(storageKey, preference)
}

watch(themePreference, applyTheme, { immediate: true })
mediaQuery.addEventListener('change', applyTheme)
