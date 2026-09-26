// Public surface of the auth feature — import from '@/features/auth' elsewhere in the app.
export { AuthProvider } from './context/AuthProvider'
export { useAuth } from './context/AuthContext'
export { GuestRoute, ProtectedRoute } from './components/RouteGuards'
export { default as LoginPage } from './pages/LoginPage'
export type { User } from './types'
