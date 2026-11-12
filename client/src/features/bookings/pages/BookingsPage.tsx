import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router'
import { CheckCircle2, Plus } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { Pagination } from '@/components/ui/Pagination'
import { SearchInput } from '@/components/ui/SearchInput'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { formatDay, formatTimeRange } from '@/lib/format'
import { BookingsTable } from '../components/BookingsTable'
import { useBookings, useCancelBooking, useSetRefund } from '../hooks'
import type { Booking, BookingListParams, BookingStatus } from '../types'
import '../bookings.css'

const PER_PAGE = 20
const WHEN_TABS = [
  { value: 'upcoming', label: 'Upcoming' },
  { value: 'past', label: 'Past' },
  { value: 'all', label: 'All' },
] as const
type WhenTab = (typeof WHEN_TABS)[number]['value']

export default function BookingsPage() {
  // Filters live in the URL so they survive reloads and can be shared.
  const [searchParams, setSearchParams] = useSearchParams()
  const page = Math.max(1, Number(searchParams.get('page')) || 1)
  const when = (searchParams.get('when') ?? 'upcoming') as WhenTab
  const status = (searchParams.get('status') ?? '') as BookingStatus | ''
  const [searchInput, setSearchInput] = useState(searchParams.get('q') ?? '')
  const search = useDebouncedValue(searchInput.trim(), 300)

  // One-off message from the booking form ("Booking #123 created")
  const location = useLocation()
  const navigate = useNavigate()
  const [flash] = useState<string | null>(
    () => (location.state as { flash?: string } | null)?.flash ?? null,
  )
  useEffect(() => {
    if (flash) navigate(location.pathname + location.search, { replace: true, state: null })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  function updateParams(changes: Record<string, string | null>, resetPage = true) {
    setSearchParams(
      (prev) => {
        const next = new URLSearchParams(prev)
        for (const [k, v] of Object.entries(changes)) {
          if (v) next.set(k, v)
          else next.delete(k)
        }
        if (resetPage) next.delete('page')
        return next
      },
      { replace: true },
    )
  }

  useEffect(() => {
    if ((searchParams.get('q') ?? '') !== search) updateParams({ q: search || null })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search])

  const params: BookingListParams = {
    search,
    when: when === 'all' ? undefined : when,
    status: status || undefined,
    page,
    per_page: PER_PAGE,
  }
  const { data, isPending, isError, error, isFetching } = useBookings(params)

  const [toCancel, setToCancel] = useState<Booking | null>(null)
  const [toRefund, setToRefund] = useState<Booking | null>(null)
  const setRefund = useSetRefund()
  const cancelBooking = useCancelBooking()

  return (
    <>
      <PageHeader
        title="Bookings"
        description="Bookings made by players in the app and by staff here. Synced with Bookeo."
        actions={
          <Link to="/bookings/new" className="btn btn-primary">
            <Plus size={16} aria-hidden="true" />
            New booking
          </Link>
        }
      />

      {flash && (
        <div className="notice notice-success" role="status">
          <CheckCircle2 size={18} aria-hidden="true" />
          {flash}
        </div>
      )}

      <div className="panel">
        <div className="panel-toolbar panel-toolbar-wrap">
          <SearchInput
            value={searchInput}
            onChange={setSearchInput}
            placeholder="Search player or booking #"
          />
          <div className="segmented" role="tablist" aria-label="Time">
            {WHEN_TABS.map((t) => (
              <button
                key={t.value}
                type="button"
                role="tab"
                aria-selected={when === t.value}
                className={when === t.value ? 'active' : undefined}
                onClick={() => updateParams({ when: t.value === 'upcoming' ? null : t.value })}
              >
                {t.label}
              </button>
            ))}
          </div>
          <select
            aria-label="Status"
            value={status}
            onChange={(e) => updateParams({ status: e.target.value || null })}
          >
            <option value="">All statuses</option>
            <option value="confirmed">Confirmed</option>
            <option value="pending">Pending</option>
            <option value="canceled">Canceled</option>
          </select>
          {isFetching && !isPending && <span className="muted small">Updating…</span>}
        </div>

        {isPending ? (
          <p className="panel-empty muted">Loading bookings…</p>
        ) : isError ? (
          <p className="panel-empty error">{error.message}</p>
        ) : data.items.length === 0 ? (
          <p className="panel-empty muted">
            {search || status || when !== 'upcoming'
              ? 'No bookings match these filters.'
              : 'No upcoming bookings.'}
          </p>
        ) : (
          <>
            <BookingsTable
              bookings={data.items}
              onCancel={(b) => {
                cancelBooking.reset()
                setToCancel(b)
              }}
              onRefund={(b) => {
                setRefund.reset()
                setToRefund(b)
              }}
            />
            <Pagination
              page={data.page}
              perPage={data.per_page}
              total={data.total}
              onPageChange={(p) => updateParams({ page: p > 1 ? String(p) : null }, false)}
            />
          </>
        )}
      </div>

      <ConfirmDialog
        open={toCancel !== null}
        title="Cancel booking"
        confirmLabel="Cancel booking"
        cancelLabel="Keep booking"
        message={
          toCancel && (
            <>
              Cancel booking <strong>#{toCancel.booking_number}</strong> for{' '}
              <strong>{toCancel.player.full_name}</strong> — {toCancel.game_name},{' '}
              {formatDay(toCancel.start_time)} {formatTimeRange(toCancel.start_time, toCancel.end_time)}
              ? The seats are released in Bookeo. The booking stays listed as canceled.
              {Number(toCancel.amount_paid) > 0 && (
                <>
                  {' '}
                  <strong>
                    {toCancel.price?.currency ?? 'QAR'} {Number(toCancel.amount_paid).toFixed(2)} was paid online:
                  </strong>{' '}
                  the booking will show “Refund pending”. Refund it from the SkipCash merchant portal, then mark it
                  refunded here.
                </>
              )}
            </>
          )
        }
        pending={cancelBooking.isPending}
        error={cancelBooking.error?.message}
        onConfirm={() =>
          toCancel && cancelBooking.mutate(toCancel.id, { onSuccess: () => setToCancel(null) })
        }
        onClose={() => setToCancel(null)}
      />

      <ConfirmDialog
        open={toRefund !== null}
        title={toRefund?.refund_status === 'pending' ? 'Mark refunded' : 'Undo refunded'}
        confirmLabel={toRefund?.refund_status === 'pending' ? 'Mark refunded' : 'Mark not refunded'}
        cancelLabel="Back"
        message={
          toRefund &&
          (toRefund.refund_status === 'pending' ? (
            <>
              Booking <strong>#{toRefund.booking_number}</strong> was canceled after{' '}
              {toRefund.price?.currency ?? 'QAR'} {Number(toRefund.amount_paid).toFixed(2)} was paid. Mark it refunded
              once the money has been returned from the SkipCash merchant portal.
            </>
          ) : (
            <>
              Set booking <strong>#{toRefund.booking_number}</strong> back to “Refund pending”?
            </>
          ))
        }
        pending={setRefund.isPending}
        error={setRefund.error?.message}
        onConfirm={() =>
          toRefund &&
          setRefund.mutate(
            { id: toRefund.id, status: toRefund.refund_status === 'pending' ? 'refunded' : 'pending' },
            { onSuccess: () => setToRefund(null) },
          )
        }
        onClose={() => setToRefund(null)}
      />
    </>
  )
}
