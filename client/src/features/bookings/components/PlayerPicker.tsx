import { useEffect, useId, useRef, useState } from 'react'
import type { KeyboardEvent } from 'react'
import { Search } from 'lucide-react'
import { usePlayers } from '@/features/players'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'

export type PickedPlayer = { id: string; full_name: string; playername: string }

type PlayerPickerProps = {
  value: PickedPlayer | null
  onChange: (player: PickedPlayer | null) => void
  /** Player ids that can't be picked (e.g. already in the group). */
  exclude?: string[]
  placeholder?: string
}

const RESULTS = 8

/** Searchable player combobox backed by the players API. */
export function PlayerPicker({
  value,
  onChange,
  exclude = [],
  placeholder = 'Search by playername or name',
}: PlayerPickerProps) {
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const search = useDebouncedValue(query.trim(), 250)
  // Fetch extra rows so excluded players don't leave the list short.
  const { data, isFetching } = usePlayers({
    search,
    page: 1,
    per_page: Math.min(RESULTS + exclude.length, 100),
  })
  const listId = useId()
  // Closing is delayed on blur so a click on an option still registers; refocusing must
  // cancel that pending close, or the list would shut right after it reopens.
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const cancelClose = () => {
    if (closeTimer.current) clearTimeout(closeTimer.current)
    closeTimer.current = null
  }
  useEffect(() => cancelClose, [])

  if (value) {
    return (
      <div className="picked">
        <div className="stack-cell">
          <span className="strong">{value.full_name}</span>
          <span className="muted">@{value.playername}</span>
        </div>
        <button type="button" className="btn btn-secondary btn-sm" onClick={() => onChange(null)}>
          Change
        </button>
      </div>
    )
  }

  const options = (data?.items ?? []).filter((p) => !exclude.includes(p.id)).slice(0, RESULTS)

  function pick(index: number) {
    const p = options[index]
    if (!p) return
    onChange({ id: p.id, full_name: p.full_name, playername: p.playername })
    setQuery('')
    setOpen(false)
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setOpen(true)
      setActive((i) => Math.min(i + 1, options.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActive((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter' && open) {
      e.preventDefault()
      pick(active)
    } else if (e.key === 'Escape') {
      setOpen(false)
    }
  }

  return (
    <div className="combobox">
      <label className="search-input">
        <Search size={16} aria-hidden="true" />
        <input
          type="text"
          role="combobox"
          aria-label={placeholder}
          aria-expanded={open}
          aria-controls={listId}
          placeholder={placeholder}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setActive(0)
            setOpen(true)
          }}
          onFocus={() => {
            cancelClose()
            setOpen(true)
          }}
          onBlur={() => {
            cancelClose()
            closeTimer.current = setTimeout(() => setOpen(false), 150)
          }}
          onKeyDown={onKeyDown}
        />
      </label>
      {open && (
        <ul className="combobox-list" id={listId} role="listbox">
          {options.length === 0 ? (
            <li className="combobox-empty muted">{isFetching ? 'Searching…' : 'No players found'}</li>
          ) : (
            options.map((p, i) => (
              <li
                key={p.id}
                role="option"
                aria-selected={i === active}
                className={i === active ? 'active' : undefined}
                onMouseEnter={() => setActive(i)}
                onMouseDown={(e) => {
                  e.preventDefault()
                  pick(i)
                }}
              >
                <span className="strong">{p.full_name}</span>
                <span className="muted">@{p.playername}</span>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  )
}
