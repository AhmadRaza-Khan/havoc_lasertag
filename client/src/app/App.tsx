import { QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter } from 'react-router'
import { AuthProvider } from '@/features/auth'
import { isPublicSite } from '@/lib/config'
import { queryClient } from '@/lib/query-client'
import { AppRoutes, PublicRoutes } from './router'

export default function App() {
  // The policy site never loads the staff session or the admin screens.
  if (isPublicSite()) {
    return (
      <BrowserRouter>
        <PublicRoutes />
      </BrowserRouter>
    )
  }
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </AuthProvider>
    </QueryClientProvider>
  )
}
