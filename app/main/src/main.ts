import { devtools } from '@vue/devtools'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { i18n } from './i18n'

import './vue_lib/assets/main.postcss'

import App from './App.vue'

if (process.env.NODE_ENV === 'development') {
	devtools.connect('http://localhost', 8098)
}

const supportedLocales = ['en', 'pt-BR', 'de', 'es', 'fr', 'it']
const savedLocale = localStorage.getItem('rquickshare-locale')
const systemLanguage = navigator.language.toLowerCase()
const detectedLocale = systemLanguage.startsWith('pt') ? 'pt-BR' : systemLanguage.split('-')[0]
i18n.global.locale.value = savedLocale && supportedLocales.includes(savedLocale)
  ? savedLocale
  : supportedLocales.includes(detectedLocale) ? detectedLocale : 'en'

const pinia = createPinia();
const app = createApp(App)

app.use(pinia);
app.use(i18n);

app.mount('#app')
