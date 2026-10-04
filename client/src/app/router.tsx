import { Navigate, Route, Routes } from 'react-router'
import { AdminLayout } from '@/components/layout/AdminLayout'
import { GuestRoute, LoginPage, ProtectedRoute } from '@/features/auth'
import { BookingFormPage, BookingsPage } from '@/features/bookings'
import { DashboardPage } from '@/features/dashboard'
import { OffersPage } from '@/features/offers'
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
          <Route path="/bookings" element={<BookingsPage />} />
          <Route path="/bookings/new" element={<BookingFormPage />} />
          <Route path="/bookings/:id/edit" element={<BookingFormPage />} />
          <Route path="/offers" element={<OffersPage />} />
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
