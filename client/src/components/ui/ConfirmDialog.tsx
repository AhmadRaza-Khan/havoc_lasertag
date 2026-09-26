import type { ReactNode } from 'react'
import { Modal } from './Modal'

type ConfirmDialogProps = {
  open: boolean
  title: string
  message: ReactNode
  confirmLabel?: string
  cancelLabel?: string
  pending?: boolean
  error?: string | null
  onConfirm: () => void
  onClose: () => void
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = 'Delete',
  cancelLabel = 'Cancel',
  pending = false,
  error,
  onConfirm,
  onClose,
}: ConfirmDialogProps) {
  return (
    <Modal
      open={open}
      title={title}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose} disabled={pending}>
            {cancelLabel}
          </button>
          <button type="button" className="btn btn-danger" onClick={onConfirm} disabled={pending}>
            {pending ? 'Working…' : confirmLabel}
          </button>
        </>
      }
    >
      <p className="muted">{message}</p>
      {error && <p className="error">{error}</p>}
    </Modal>
  )
}
