import { useRef, useState } from 'react'
import type { DragEvent } from 'react'
import { AlertTriangle, CheckCircle2, FileUp, Loader2, XCircle } from 'lucide-react'
import { formatDay, formatTimeRange } from '@/lib/format'
import { useImport, useRecentImports, useUploadStats } from '../hooks'
import type { StatImport } from '../types'

const MAX_MB = 25

/** Upload a game report (PDF) and follow its import. */
export function StatsUpload() {
  const input = useRef<HTMLInputElement>(null)
  const [dragging, setDragging] = useState(false)
  const [localError, setLocalError] = useState<string | null>(null)
  const [importId, setImportId] = useState<string | null>(null)
  const upload = useUploadStats()
  const current = useImport(importId)
  const recent = useRecentImports()

  function start(file: File | undefined) {
    setLocalError(null)
    if (!file) return
    if (!file.name.toLowerCase().endsWith('.pdf')) {
      setLocalError('Please choose a PDF file (e.g. Total_statistic_17:43.pdf).')
      return
    }
    if (file.size > MAX_MB * 1024 * 1024) {
      setLocalError(`The file is larger than ${MAX_MB} MB.`)
      return
    }
    upload.mutate(file, { onSuccess: (imp) => setImportId(imp.id) })
  }

  function onDrop(e: DragEvent) {
    e.preventDefault()
    setDragging(false)
    start(e.dataTransfer.files[0])
  }

  const busy = upload.isPending || current.data?.status === 'processing'
  const history = (recent.data ?? []).filter((i) => i.id !== importId).slice(0, 5)

  return (
    <section className="panel panel-padded stats-upload">
      <div className="stats-upload-head">
        <div>
          <h2>Upload game report</h2>
          <p className="muted small">
            The arena&apos;s statistics PDF. Each player&apos;s stats are added to their record;
            the file is deleted once it has been read.
          </p>
        </div>
      </div>

      <div
        className={`dropzone${dragging ? ' dragging' : ''}${busy ? ' busy' : ''}`}
        onDragOver={(e) => {
          e.preventDefault()
          if (!busy) setDragging(true)
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={busy ? (e) => e.preventDefault() : onDrop}
      >
        <FileUp size={28} aria-hidden="true" />
        <p>
          Drag the PDF here, or{' '}
          <button
            type="button"
            className="link-button"
            disabled={busy}
            onClick={() => input.current?.click()}
          >
            choose a file
          </button>
        </p>
        <p className="muted small">PDF, up to {MAX_MB} MB</p>
        <input
          ref={input}
          type="file"
          accept="application/pdf,.pdf"
          hidden
          aria-label="Game report PDF"
          onChange={(e) => {
            start(e.target.files?.[0])
            e.target.value = '' // allow choosing the same file again
          }}
        />
      </div>

      {localError && <p className="error">{localError}</p>}
      {upload.error && <p className="error">{upload.error.message}</p>}
      {current.data && <ImportResult imp={current.data} />}

      {history.length > 0 && (
        <div className="import-history">
          <h3 className="small">Recent uploads</h3>
          <ul>
            {history.map((i) => (
              <li key={i.id}>
                <StatusIcon status={i.status} />
                <span className="import-file" title={i.source_filename}>
                  {i.source_filename}
                </span>
                <span className="muted small">
                  {i.game
                    ? `game ${formatDay(i.game.started_at)} ${formatTimeRange(i.game.started_at, i.game.ended_at)}`
                    : i.status === 'failed'
                      ? i.error
                      : 'processing…'}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  )
}

function ImportResult({ imp }: { imp: StatImport }) {
  if (imp.status === 'processing') {
    return (
      <div className="import-result import-processing" role="status">
        <Loader2 size={18} className="spin" aria-hidden="true" />
        <span>
          Reading <strong>{imp.source_filename}</strong> — this takes about half a minute…
        </span>
      </div>
    )
  }
  if (imp.status === 'failed') {
    return (
      <div className="import-result import-failed" role="alert">
        <XCircle size={18} aria-hidden="true" />
        <span>
          <strong>Nothing was saved.</strong> {imp.error}
        </span>
      </div>
    )
  }
  return (
    <div className="import-result import-done" role="status">
      <div className="import-result-line">
        <CheckCircle2 size={18} aria-hidden="true" />
        <span>
          Game of{' '}
          <strong>
            {imp.game && `${formatDay(imp.game.started_at)}, ${formatTimeRange(imp.game.started_at, imp.game.ended_at)}`}
          </strong>{' '}
          imported: {imp.matched_players.length} player
          {imp.matched_players.length === 1 ? '' : 's'} updated.
        </span>
      </div>
      {imp.unmatched_names.length > 0 && (
        <p className="import-note">
          <AlertTriangle size={16} aria-hidden="true" />
          <span>
            Not in the system: <strong>{imp.unmatched_names.join(', ')}</strong>. Their stats were
            kept — create players with these playernames and the stats are linked automatically.
          </span>
        </p>
      )}
      {imp.warnings.length > 0 && (
        <details className="import-warnings">
          <summary>
            {imp.warnings.length} note{imp.warnings.length === 1 ? '' : 's'} from reading the file
          </summary>
          <ul>
            {imp.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </details>
      )}
    </div>
  )
}

function StatusIcon({ status }: { status: StatImport['status'] }) {
  if (status === 'completed') return <CheckCircle2 size={16} className="ok" aria-label="Imported" />
  if (status === 'failed') return <XCircle size={16} className="bad" aria-label="Failed" />
  return <Loader2 size={16} className="spin" aria-label="Processing" />
}
