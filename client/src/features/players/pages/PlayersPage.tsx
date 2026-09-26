import { useEffect, useState } from 'react'
import { useSearchParams } from 'react-router'
import { Plus } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { Pagination } from '@/components/ui/Pagination'
import { SearchInput } from '@/components/ui/SearchInput'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { PlayerFormModal } from '../components/PlayerFormModal'
import { PlayersTable } from '../components/PlayersTable'
import { useDeletePlayer, usePlayers } from '../hooks'
import type { Player } from '../types'
import '../players.css'

const PER_PAGE = 20

type FormState = { open: false } | { open: true; player: Player | null }

export default function PlayersPage() {
  // Search and page live in the URL so they survive reloads and can be shared.
  const [searchParams, setSearchParams] = useSearchParams()
  const page = Math.max(1, Number(searchParams.get('page')) || 1)
  const [searchInput, setSearchInput] = useState(searchParams.get('q') ?? '')
  const search = useDebouncedValue(searchInput.trim(), 300)

  useEffect(() => {
    setSearchParams(
      (prev) => {
        if ((prev.get('q') ?? '') === search) return prev
        const next = new URLSearchParams(prev)
        if (search) next.set('q', search)
        else next.delete('q')
        next.delete('page')
        return next
      },
      { replace: true },
    )
  }, [search, setSearchParams])

  const { data, isPending, isError, error, isFetching } = usePlayers({
    search,
    page,
    per_page: PER_PAGE,
  })

  const [form, setForm] = useState<FormState>({ open: false })
  const [toDelete, setToDelete] = useState<Player | null>(null)
  const deletePlayer = useDeletePlayer()

  function setPage(p: number) {
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev)
      if (p > 1) next.set('page', String(p))
      else next.delete('page')
      return next
    })
  }

  function confirmDelete() {
    if (!toDelete) return
    deletePlayer.mutate(toDelete.id, {
      onSuccess: () => {
        setToDelete(null)
        // Step back a page if we just deleted the last row on this page
        if (data && data.items.length === 1 && page > 1) setPage(page - 1)
      },
    })
  }

  return (
    <>
      <PageHeader
        title="Players"
        description="Manage player accounts."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setForm({ open: true, player: null })}
          >
            <Plus size={16} aria-hidden="true" />
            Add player
          </button>
        }
      />

      <div className="panel">
        <div className="panel-toolbar">
          <SearchInput
            value={searchInput}
            onChange={setSearchInput}
            placeholder="Search by playername or name"
          />
          {isFetching && !isPending && <span className="muted small">Updating…</span>}
        </div>

        {isPending ? (
          <p className="panel-empty muted">Loading players…</p>
        ) : isError ? (
          <p className="panel-empty error">{error.message}</p>
        ) : data.items.length === 0 ? (
          <p className="panel-empty muted">
            {search ? `No players match “${search}”.` : 'No players yet. Add the first one.'}
          </p>
        ) : (
          <>
            <PlayersTable
              players={data.items}
              onEdit={(player) => setForm({ open: true, player })}
              onDelete={(player) => {
                deletePlayer.reset()
                setToDelete(player)
              }}
            />
            <Pagination
              page={data.page}
              perPage={data.per_page}
              total={data.total}
              onPageChange={setPage}
            />
          </>
        )}
      </div>

      <PlayerFormModal
        open={form.open}
        player={form.open ? form.player : null}
        onClose={() => setForm({ open: false })}
      />

      <ConfirmDialog
        open={toDelete !== null}
        title="Delete player"
        message={
          <>
            Delete <strong>{toDelete?.full_name}</strong> (@{toDelete?.playername})? This cannot be
            undone.
          </>
        }
        pending={deletePlayer.isPending}
        error={deletePlayer.error?.message}
        onConfirm={confirmDelete}
        onClose={() => setToDelete(null)}
      />
    </>
  )
}
