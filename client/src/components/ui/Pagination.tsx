import { ChevronLeft, ChevronRight } from 'lucide-react'

type PaginationProps = {
  page: number
  perPage: number
  total: number
  onPageChange: (page: number) => void
}

export function Pagination({ page, perPage, total, onPageChange }: PaginationProps) {
  const pageCount = Math.max(1, Math.ceil(total / perPage))
  const from = total === 0 ? 0 : (page - 1) * perPage + 1
  const to = Math.min(page * perPage, total)

  return (
    <div className="pagination">
      <span className="muted">
        {from}–{to} of {total}
      </span>
      <div className="pagination-buttons">
        <button
          type="button"
          className="icon-btn"
          aria-label="Previous page"
          disabled={page <= 1}
          onClick={() => onPageChange(page - 1)}
        >
          <ChevronLeft size={18} />
        </button>
        <span className="pagination-page">
          Page {page} of {pageCount}
        </span>
        <button
          type="button"
          className="icon-btn"
          aria-label="Next page"
          disabled={page >= pageCount}
          onClick={() => onPageChange(page + 1)}
        >
          <ChevronRight size={18} />
        </button>
      </div>
    </div>
  )
}
