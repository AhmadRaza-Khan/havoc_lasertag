import { CalendarDays, LayoutDashboard, Megaphone, Settings, Users } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'

export type NavItem = {
  to: string
  label: string
  icon: LucideIcon
}

/**
 * Sidebar entries, in display order. When adding a feature, add its route in `router.tsx`
 * and its entry here.
 */
export const navItems: NavItem[] = [
  { to: '/', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/players', label: 'Players', icon: Users },
  { to: '/bookings', label: 'Bookings', icon: CalendarDays },
  { to: '/offers', label: 'Offers', icon: Megaphone },
  { to: '/settings', label: 'Settings', icon: Settings },
]
