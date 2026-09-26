import { Navigate, Route, Routes } from 'react-router'
import { AdminLayout } from '@/components/layout/AdminLayout'
import { GuestRoute, LoginPage, ProtectedRoute } from '@/features/auth'
import { DashboardPage } from '@/features/dashboard'
import { PlayersPage } from '@/features/players'

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<GuestRoute />}>
        <Route path="/login" element={<LoginPage />} />
      </Route>

      {/* Authenticated admin area. Keep in sync with the sidebar in navigation.ts */}
      <Route element={<ProtectedRoute />}>
        <Route element={<AdminLayout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/players" element={<PlayersPage />} />
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
