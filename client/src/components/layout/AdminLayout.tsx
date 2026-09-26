import { Outlet } from 'react-router'
import { navItems } from '@/app/navigation'
import { Sidebar } from './Sidebar'
import './layout.css'

/** Shell for all authenticated pages: sidebar + the selected feature's page. */
export function AdminLayout() {
  return (
    <div className="admin-layout">
      <Sidebar items={navItems} />
      <main className="admin-main">
        <Outlet />
      </main>
    </div>
  )
}
