import { NavLink, useNavigate } from 'react-router'
import { LogOut } from 'lucide-react'
import type { NavItem } from '@/app/navigation'
import { useAuth } from '@/features/auth'

export function Sidebar({ items }: { items: NavItem[] }) {
  const { user, logout } = useAuth()
  const navigate = useNavigate()

  async function handleLogout() {
    await logout()
    navigate('/login', { replace: true })
  }

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="sidebar-logo" aria-hidden="true">
          H
        </span>
        <span>Havoc Admin</span>
      </div>

      <nav className="sidebar-nav">
        {items.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) => `sidebar-link${isActive ? ' active' : ''}`}
          >
            <Icon size={18} aria-hidden="true" />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        <div className="sidebar-user">
          <span className="sidebar-user-name">{user?.name}</span>
          <span className="sidebar-user-handle">@{user?.username}</span>
        </div>
        <button type="button" className="icon-btn" aria-label="Log out" title="Log out" onClick={handleLogout}>
          <LogOut size={18} />
        </button>
      </div>
    </aside>
  )
}
