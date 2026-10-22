import { Navigate, Route, Routes } from 'react-router'
import { AdminLayout } from '@/components/layout/AdminLayout'
import { GuestRoute, LoginPage, ProtectedRoute } from '@/features/auth'
import { BookingFormPage, BookingsPage } from '@/features/bookings'
import { ChatsPage } from '@/features/chats'
import { DashboardPage } from '@/features/dashboard'
import { DeleteAccountPage, PrivacyPage, TermsPage } from '@/features/legal'
import { OffersPage } from '@/features/offers'
import { PlayersPage } from '@/features/players'
import { PlayerStatsPage } from '@/features/stats'
import { SettingsPage } from '@/features/settings'

export function AppRoutes() {
  return (
    <Routes>
      {/* Public: store listings and the app link here (no login) */}
      <Route path="/privacy" element={<PrivacyPage />} />
      <Route path="/terms" element={<TermsPage />} />
      <Route path="/delete-account" element={<DeleteAccountPage />} />

      <Route element={<GuestRoute />}>
        <Route path="/login" element={<LoginPage />} />
      </Route>

      {/* Authenticated admin area. Keep in sync with the sidebar in navigation.ts */}
      <Route element={<ProtectedRoute />}>
        <Route element={<AdminLayout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/players" element={<PlayersPage />} />
          <Route path="/players/:id/stats" element={<PlayerStatsPage />} />
          <Route path="/bookings" element={<BookingsPage />} />
          <Route path="/bookings/new" element={<BookingFormPage />} />
          <Route path="/bookings/:id/edit" element={<BookingFormPage />} />
          <Route path="/chats" element={<ChatsPage />} />
          <Route path="/offers" element={<OffersPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Route>

      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
