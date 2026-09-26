import { Navigate, Outlet, useLocation } from 'react-router'
import { useAuth } from '../context/AuthContext'

/** Renders child routes only for logged-in users; otherwise redirects to /login. */
export function ProtectedRoute() {
  const { user, loading } = useAuth()
  const location = useLocation()

  if (loading) return <div className="center muted">Loading…</div>
  if (!user) return <Navigate to="/login" replace state={{ from: location }} />
  return <Outlet />
}

/**
 * Renders child routes only for logged-out users (e.g. the login page).
 * Once logged in, sends the user back to the page they originally requested.
 */
export function GuestRoute() {
  const { user, loading } = useAuth()
  const location = useLocation()

  if (loading) return <div className="center muted">Loading…</div>
  if (user) {
    const from = (location.state as { from?: { pathname: string; search: string } } | null)?.from
    return <Navigate to={from ? from.pathname + from.search : '/'} replace />
  }
  return <Outlet />
}
