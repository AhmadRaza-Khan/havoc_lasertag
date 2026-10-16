import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from 'react'
import { useSearchParams } from 'react-router'
import { ArrowLeft, Check, CheckCheck, MessageCircle, Send } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { Pagination } from '@/components/ui/Pagination'
import { SearchInput } from '@/components/ui/SearchInput'
import { apiUrl } from '@/lib/api-client'
import { formatDay, formatTime } from '@/lib/format'
import { chatsApi } from '../api'
import { useChatPlayers, useConversations, useMarkRead, useMessages, useSendMessage } from '../hooks'
import type { ChatMessage, ChatPlayer, Conversation } from '../types'
import '../chats.css'

const MAX_CHARS = 2000

/** Chat with players as "Havoc Admin": conversations and every player on the left, the chat on the right. */
export default function ChatsPage() {
  const [params, setParams] = useSearchParams()
  const selectedId = params.get('player')
  const [tab, setTab] = useState<'chats' | 'players'>('chats')
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  // The player picked from "All players" (they may have no conversation yet).
  const [picked, setPicked] = useState<ChatPlayer | null>(null)

  const conversations = useConversations()
  const players = useChatPlayers(search.trim(), page, tab === 'players')

  const term = search.trim().toLowerCase()
  const shownConversations = (conversations.data ?? []).filter(
    (c) =>
      !term || c.player.playername.toLowerCase().includes(term) || c.player.full_name.toLowerCase().includes(term),
  )
  const selected: ChatPlayer | null =
    conversations.data?.find((c) => c.player.id === selectedId)?.player ??
    players.data?.items.find((p) => p.id === selectedId) ??
    (picked?.id === selectedId ? picked : null)

  function open(player: ChatPlayer) {
    setPicked(player)
    setParams({ player: player.id })
  }

  return (
    <>
      <PageHeader
        title="Chats"
        description="Message any player. Players see your messages from “Havoc Admin” in the app."
      />
      <div className={`chats${selectedId ? ' has-selection' : ''}`}>
        <aside className="chats-list panel">
          <div className="chats-list-head">
            <div className="tabs chats-tabs" role="tablist">
              <button
                type="button"
                role="tab"
                aria-selected={tab === 'chats'}
                className={`tab${tab === 'chats' ? ' active' : ''}`}
                onClick={() => setTab('chats')}
              >
                Chats
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={tab === 'players'}
                className={`tab${tab === 'players' ? ' active' : ''}`}
                onClick={() => setTab('players')}
              >
                All players
              </button>
            </div>
            <SearchInput
              value={search}
              onChange={(v) => {
                setSearch(v)
                setPage(1)
              }}
              placeholder="Search players"
            />
          </div>

          {tab === 'chats' ? (
            conversations.isPending ? (
              <p className="chats-empty muted">Loading chats…</p>
            ) : conversations.isError ? (
              <p className="chats-empty error">{conversations.error.message}</p>
            ) : shownConversations.length === 0 ? (
              <p className="chats-empty muted">
                {term ? 'No chats match.' : 'No chats yet. Open “All players” to message someone.'}
              </p>
            ) : (
              <ul className="chats-items">
                {shownConversations.map((c) => (
                  <ConversationItem
                    key={c.id}
                    c={c}
                    active={c.player.id === selectedId}
                    onOpen={() => open(c.player)}
                  />
                ))}
              </ul>
            )
          ) : players.isPending ? (
            <p className="chats-empty muted">Loading players…</p>
          ) : players.isError ? (
            <p className="chats-empty error">{players.error.message}</p>
          ) : players.data.items.length === 0 ? (
            <p className="chats-empty muted">No players match.</p>
          ) : (
            <>
              <ul className="chats-items">
                {players.data.items.map((p) => (
                  <li key={p.id}>
                    <button
                      type="button"
                      className={`chats-item${p.id === selectedId ? ' active' : ''}`}
                      onClick={() => open(p)}
                    >
                      <Avatar player={p} />
                      <span className="chats-item-main">
                        <span className="chats-item-name">{p.full_name}</span>
                        <span className="chats-item-preview">@{p.playername}</span>
                      </span>
                      <span className="chats-item-side muted small">{presence(p)}</span>
                    </button>
                  </li>
                ))}
              </ul>
              <Pagination
                page={players.data.page}
                perPage={players.data.per_page}
                total={players.data.total}
                onPageChange={setPage}
              />
            </>
          )}
        </aside>

        <section className="chats-thread panel">
          {selectedId && selected ? (
            <Thread key={selectedId} player={selected} onBack={() => setParams({})} />
          ) : selectedId ? (
            <p className="chats-placeholder muted">Loading…</p>
          ) : (
            <div className="chats-placeholder">
              <MessageCircle size={40} aria-hidden="true" />
              <p>Pick a chat or a player to start messaging.</p>
            </div>
          )}
        </section>
      </div>
    </>
  )
}

function ConversationItem({ c, active, onOpen }: { c: Conversation; active: boolean; onOpen: () => void }) {
  const last = c.last_message
  const fromUs = last && last.sender_id !== c.player.id
  return (
    <li>
      <button type="button" className={`chats-item${active ? ' active' : ''}`} onClick={onOpen}>
        <Avatar player={c.player} />
        <span className="chats-item-main">
          <span className="chats-item-name">{c.player.full_name}</span>
          <span className={`chats-item-preview${c.unread_count ? ' unread' : ''}`}>
            {last ? `${fromUs ? 'You: ' : ''}${last.body}` : `@${c.player.playername}`}
          </span>
        </span>
        <span className="chats-item-side">
          {last && <span className="muted small">{stamp(last.created_at)}</span>}
          {c.unread_count > 0 && <span className="chats-unread">{c.unread_count}</span>}
        </span>
      </button>
    </li>
  )
}

function Thread({ player, onBack }: { player: ChatPlayer; onBack: () => void }) {
  const latest = useMessages(player.id)
  const send = useSendMessage(player.id)
  const markRead = useMarkRead()
  const [older, setOlder] = useState<ChatMessage[]>([])
  const [hasMoreOlder, setHasMoreOlder] = useState<boolean | null>(null)
  const [loadingOlder, setLoadingOlder] = useState(false)
  const [draft, setDraft] = useState('')
  const listRef = useRef<HTMLDivElement>(null)

  // Oldest first, without duplicates between the polled page and loaded history.
  const seen = new Set<string>()
  const messages = [...(latest.data?.items ?? []), ...older]
    .filter((m) => (seen.has(m.id) ? false : (seen.add(m.id), true)))
    .sort((a, b) => a.created_at.localeCompare(b.created_at))
  const hasMore = hasMoreOlder ?? latest.data?.has_more ?? false

  // Read what the player sent while this chat is open.
  const newestUnread = messages.findLast((m) => m.sender_id === player.id && !m.read_at)?.id
  const { mutate: markReadNow } = markRead
  useEffect(() => {
    if (newestUnread) markReadNow(player.id)
  }, [newestUnread, player.id, markReadNow])

  // Stay at the bottom when a new message arrives.
  const newestId = messages.at(-1)?.id
  useEffect(() => {
    const el = listRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [newestId])

  async function loadOlder() {
    const oldest = messages[0]
    if (!oldest) return
    setLoadingOlder(true)
    try {
      const page = await chatsApi.messages(player.id, oldest.id)
      setOlder((prev) => [...prev, ...page.items])
      setHasMoreOlder(page.has_more)
    } finally {
      setLoadingOlder(false)
    }
  }

  function submit(e?: FormEvent) {
    e?.preventDefault()
    const body = draft.trim()
    if (!body || send.isPending) return
    send.mutate(body, { onSuccess: () => setDraft('') })
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    // Enter sends, Shift+Enter adds a line.
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      submit()
    }
  }

  return (
    <>
      <header className="chats-thread-head">
        <button type="button" className="icon-btn chats-back" aria-label="Back to chats" onClick={onBack}>
          <ArrowLeft size={18} />
        </button>
        <Avatar player={player} />
        <div className="chats-thread-who">
          <span className="chats-thread-name">{player.full_name}</span>
          <span className={`small ${player.online ? 'chats-online-text' : 'muted'}`}>
            @{player.playername} · {presence(player)}
          </span>
        </div>
      </header>

      <div className="chats-messages" ref={listRef}>
        {hasMore && (
          <button type="button" className="btn btn-secondary btn-sm chats-older" onClick={loadOlder} disabled={loadingOlder}>
            {loadingOlder ? 'Loading…' : 'Load earlier messages'}
          </button>
        )}
        {latest.isPending ? (
          <p className="chats-placeholder muted">Loading messages…</p>
        ) : latest.isError ? (
          <p className="chats-placeholder error">{latest.error.message}</p>
        ) : messages.length === 0 ? (
          <div className="chats-placeholder">
            <MessageCircle size={32} aria-hidden="true" />
            <p>No messages yet. Say hi to {player.playername}.</p>
          </div>
        ) : (
          messages.map((m, i) => {
            const mine = m.sender_id !== player.id
            const day = formatDay(m.created_at)
            const newDay = i === 0 || formatDay(messages[i - 1].created_at) !== day
            return (
              <div key={m.id} className="chats-row">
                {newDay && <div className="chats-day">{day}</div>}
                <div className={`chats-bubble ${mine ? 'mine' : 'theirs'}`}>
                  {mine && <span className="chats-from">Havoc Admin</span>}
                  <p className="chats-body">{m.body}</p>
                  <span className="chats-meta">
                    {formatTime(m.created_at)}
                    {mine &&
                      (m.read_at ? (
                        <CheckCheck size={14} aria-label="Read" className="chats-read" />
                      ) : (
                        <Check size={14} aria-label="Sent" />
                      ))}
                  </span>
                </div>
              </div>
            )
          })
        )}
      </div>

      <form className="chats-composer" onSubmit={submit}>
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value.slice(0, MAX_CHARS))}
          onKeyDown={onKeyDown}
          placeholder={`Message ${player.playername} as Havoc Admin`}
          aria-label="Message"
          rows={1}
        />
        <button type="submit" className="btn btn-primary" disabled={!draft.trim() || send.isPending} aria-label="Send">
          <Send size={16} aria-hidden="true" />
          Send
        </button>
      </form>
      {send.isError && <p className="error small chats-send-error">{send.error.message}</p>}
    </>
  )
}

function Avatar({ player }: { player: ChatPlayer }) {
  const [failed, setFailed] = useState(false)
  const initials = (player.full_name || player.playername)
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase()
  return (
    <span className="chats-avatar">
      {player.avatar_url && !failed ? (
        <img src={apiUrl(player.avatar_url)} alt="" onError={() => setFailed(true)} />
      ) : (
        <span aria-hidden="true">{initials || '?'}</span>
      )}
      {player.online && <span className="chats-dot" aria-label="Online" />}
    </span>
  )
}

/** "Online", "Last seen 5 min ago", or "Not in the app yet". */
function presence(p: ChatPlayer): string {
  if (p.online) return 'Online'
  if (!p.last_seen_at) return 'Not in the app yet'
  return `Last seen ${ago(p.last_seen_at)}`
}

function ago(iso: string): string {
  const minutes = Math.floor((Date.now() - Date.parse(iso)) / 60000)
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes} min ago`
  if (minutes < 24 * 60) return `${Math.floor(minutes / 60)} h ago`
  return formatDay(iso)
}

/** Time today, else the day. */
function stamp(iso: string): string {
  return formatDay(iso) === formatDay(new Date().toISOString()) ? formatTime(iso) : formatDay(iso)
}
