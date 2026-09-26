import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
// Global styles must load before App so feature stylesheets (imported by App's modules) can
// override them. CSS is applied in import order.
import '@/styles/global.css'
import App from '@/app/App'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
