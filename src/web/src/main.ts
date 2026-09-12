import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import './styles/tokens.css'

const app = createApp(App).use(createPinia()).use(router)

// A render error must not be a blank page. It is written where a person will see it, and to the
// console where a tool will.
app.config.errorHandler = (error, _instance, info) => {
  console.error(error, info)
  const banner = document.createElement('pre')
  banner.className = 'fatal'
  banner.textContent = `${info}: ${error instanceof Error ? (error.stack ?? error.message) : String(error)}`
  document.body.prepend(banner)
}

app.mount('#app')
