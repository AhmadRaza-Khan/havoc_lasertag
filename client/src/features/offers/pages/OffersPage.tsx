import { useEffect, useState } from 'react'
import { useSearchParams } from 'react-router'
import { Plus } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { Pagination } from '@/components/ui/Pagination'
import { SearchInput } from '@/components/ui/SearchInput'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { OfferFormModal } from '../components/OfferFormModal'
import { OffersTable } from '../components/OffersTable'
import { useDeleteOffer, useOffers } from '../hooks'
import type { Offer } from '../types'
import '../offers.css'

const PER_PAGE = 20

type FormState = { open: false } | { open: true; offer: Offer | null }

export default function OffersPage() {
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

  const { data, isPending, isError, error, isFetching } = useOffers({
    search,
    page,
    per_page: PER_PAGE,
  })

  const [form, setForm] = useState<FormState>({ open: false })
  const [toDelete, setToDelete] = useState<Offer | null>(null)
  const deleteOffer = useDeleteOffer()

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
    deleteOffer.mutate(toDelete.id, {
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
        title="Offers"
        description="Promotions shown to players in the mobile app."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setForm({ open: true, offer: null })}
          >
            <Plus size={16} aria-hidden="true" />
            Add offer
          </button>
        }
      />

      <div className="panel">
        <div className="panel-toolbar">
          <SearchInput value={searchInput} onChange={setSearchInput} placeholder="Search offers" />
          {isFetching && !isPending && <span className="muted small">Updating…</span>}
        </div>

        {isPending ? (
          <p className="panel-empty muted">Loading offers…</p>
        ) : isError ? (
          <p className="panel-empty error">{error.message}</p>
        ) : data.items.length === 0 ? (
          <p className="panel-empty muted">
            {search ? `No offers match “${search}”.` : 'No offers yet. Add the first one.'}
          </p>
        ) : (
          <>
            <OffersTable
              offers={data.items}
              onEdit={(offer) => setForm({ open: true, offer })}
              onDelete={(offer) => {
                deleteOffer.reset()
                setToDelete(offer)
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

      <OfferFormModal
        open={form.open}
        offer={form.open ? form.offer : null}
        onClose={() => setForm({ open: false })}
      />

      <ConfirmDialog
        open={toDelete !== null}
        title="Delete offer"
        message={
          <>
            Delete <strong>{toDelete?.title}</strong>? It disappears from the app right away. This
            cannot be undone.
          </>
        }
        pending={deleteOffer.isPending}
        error={deleteOffer.error?.message}
        onConfirm={confirmDelete}
        onClose={() => setToDelete(null)}
      />
    </>
  )
}
