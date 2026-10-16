import { useUnreadTotal } from '../hooks'

/** Unread messages from players, next to "Chats" in the sidebar. */
export function ChatUnreadBadge() {
  const unread = useUnreadTotal()
  if (unread === 0) return null
  return (
    <span className="nav-badge" aria-label={`${unread} unread`}>
      {unread > 99 ? '99+' : unread}
    </span>
  )
}
