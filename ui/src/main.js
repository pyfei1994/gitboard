import { createApp } from 'vue'
import App from './App.vue'
import './styles.css'
import { call, isNative } from './lib/api'

createApp(App).mount('#app')

if (isNative) {
  call('boot_ok', { info: navigator.userAgent }).catch(() => {})
  window.addEventListener('error', (event) => {
    call('js_error', { message: String(event.message || event.error || 'unknown') }).catch(() => {})
  })
  window.addEventListener('unhandledrejection', (event) => {
    call('js_error', { message: String(event.reason || 'unhandled rejection') }).catch(() => {})
  })
}
