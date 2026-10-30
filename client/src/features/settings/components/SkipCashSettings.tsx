import { useState } from 'react'
import type { FormEvent } from 'react'
import { CircleCheck, CircleX, Copy, PlugZap, TriangleAlert } from 'lucide-react'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { apiUrl } from '@/lib/api-client'
import { useRemoveSkipCash, useSaveSkipCash, useSkipCashCredentials, useTestSkipCash } from '../hooks'
import type { SkipCashCredentials, SkipCashCredentialsInput, SkipCashEnvironment } from '../types'

const WEBHOOK_URL = apiUrl('/api/webhooks/skipcash')

/** SkipCash merchant credentials: entered here, tested, and used by the server right away. */
export function SkipCashSettings() {
  const { data, isPending, isError, error } = useSkipCashCredentials()

  if (isPending) return <p className="muted">Loading SkipCash settings…</p>
  if (isError) return <p className="error">{error.message}</p>
  return <SkipCashForm saved={data} />
}

function SkipCashForm({ saved }: { saved: SkipCashCredentials }) {
  const [values, setValues] = useState<SkipCashCredentialsInput>({
    environment: saved.environment ?? 'production',
    client_id: saved.client_id ?? '',
    key_id: saved.key_id ?? '',
    key_secret: '',
    webhook_key: '',
  })
  const [savedNote, setSavedNote] = useState(false)
  const [confirmRemove, setConfirmRemove] = useState(false)
  const [copied, setCopied] = useState(false)
  const save = useSaveSkipCash()
  const remove = useRemoveSkipCash()
  const test = useTestSkipCash()

  function set<K extends keyof SkipCashCredentialsInput>(key: K, value: SkipCashCredentialsInput[K]) {
    setValues((v) => ({ ...v, [key]: value }))
    setSavedNote(false)
    test.reset()
  }

  const complete =
    values.client_id.trim() !== '' &&
    values.key_id.trim() !== '' &&
    (saved.key_secret_set || values.key_secret.trim() !== '') &&
    (saved.webhook_key_set || values.webhook_key.trim() !== '')
  const busy = save.isPending || test.isPending || remove.isPending
  // What's in the form isn't what the server uses yet.
  const unsaved =
    !saved.configured ||
    values.environment !== saved.environment ||
    values.client_id.trim() !== saved.client_id ||
    values.key_id.trim() !== saved.key_id ||
    values.key_secret.trim() !== '' ||
    values.webhook_key.trim() !== ''

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate(values, {
      onSuccess: () => {
        // The secrets are saved now; the fields go back to "leave empty to keep".
        setValues((v) => ({ ...v, key_secret: '', webhook_key: '' }))
        setSavedNote(true)
      },
    })
  }

  function copyWebhook() {
    void navigator.clipboard.writeText(WEBHOOK_URL).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  const secretPlaceholder = (isSet: boolean) => (isSet ? 'Saved — leave empty to keep it' : '')

  return (
    <form className="settings-section" onSubmit={handleSubmit} autoComplete="off">
      <div className="settings-heading">
        <h2>SkipCash</h2>
        {saved.configured ? (
          <span className="badge badge-confirmed">
            Connected · {saved.environment === 'sandbox' ? 'Sandbox' : 'Production'}
          </span>
        ) : (
          <span className="badge badge-pending">Not set up</span>
        )}
      </div>
      <p className="muted">
        From the SkipCash merchant portal → Credentials. Changes apply immediately, no restart
        needed.
      </p>

      {savedNote && !save.isPending && (
        <p className="notice notice-success" role="status">
          <CircleCheck size={16} aria-hidden="true" />
          Saved. Online payment is on and uses these credentials.
        </p>
      )}

      <div className="segmented" role="radiogroup" aria-label="SkipCash environment">
        {(['production', 'sandbox'] as SkipCashEnvironment[]).map((env) => (
          <button
            key={env}
            type="button"
            role="radio"
            aria-checked={values.environment === env}
            className={values.environment === env ? 'active' : undefined}
            onClick={() => set('environment', env)}
          >
            {env === 'production' ? 'Production' : 'Sandbox (test)'}
          </button>
        ))}
      </div>
      {values.environment === 'sandbox' && (
        <p className="notice notice-warning">
          <TriangleAlert size={16} aria-hidden="true" />
          Sandbox: customers can't pay with real cards. Use it only for testing.
        </p>
      )}

      <div className="form-row">
        <label>
          Client ID
          <input
            className="mono"
            required
            spellCheck={false}
            value={values.client_id}
            onChange={(e) => set('client_id', e.target.value)}
          />
        </label>
        <label>
          Key ID
          <input
            className="mono"
            required
            spellCheck={false}
            value={values.key_id}
            onChange={(e) => set('key_id', e.target.value)}
          />
        </label>
      </div>
      <div className="form-row">
        <label>
          Key secret
          <input
            className="mono"
            type="password"
            autoComplete="new-password"
            required={!saved.key_secret_set}
            placeholder={secretPlaceholder(saved.key_secret_set)}
            value={values.key_secret}
            onChange={(e) => set('key_secret', e.target.value)}
          />
        </label>
        <label>
          Webhook key
          <input
            className="mono"
            type="password"
            autoComplete="new-password"
            required={!saved.webhook_key_set}
            placeholder={secretPlaceholder(saved.webhook_key_set)}
            value={values.webhook_key}
            onChange={(e) => set('webhook_key', e.target.value)}
          />
        </label>
      </div>

      <label>
        Webhook URL
        <span className="copy-field">
          <input className="mono" readOnly value={WEBHOOK_URL} onFocus={(e) => e.target.select()} />
          <button type="button" className="btn btn-secondary" onClick={copyWebhook}>
            <Copy size={16} aria-hidden="true" />
            {copied ? 'Copied' : 'Copy'}
          </button>
        </span>
        <span className="field-hint muted">
          Paste this into the merchant portal's webhook setting, so payments finished with the app
          closed are still confirmed.
        </span>
      </label>

      {test.data && (
        <div className={`test-result ${test.data.ok ? 'test-ok' : 'test-failed'}`} role="status">
          <strong>{test.data.ok ? 'Connection works' : 'Connection failed'}</strong>
          <ul>
            {test.data.checks.map((c) => (
              <li key={c.name}>
                {c.ok ? (
                  <CircleCheck size={16} aria-label="Passed" />
                ) : (
                  <CircleX size={16} aria-label="Failed" />
                )}
                <span>
                  <strong>{c.name}:</strong> {c.message}
                </span>
              </li>
            ))}
          </ul>
          <span className="muted small">
            The webhook key can't be tested from here; SkipCash uses it when it calls the webhook URL.
          </span>
          {test.data.ok && unsaved && (
            <span className="test-save-hint">
              <TriangleAlert size={16} aria-hidden="true" />
              Not saved yet: press Save to turn on online payment with these credentials.
            </span>
          )}
        </div>
      )}

      {(save.error ?? test.error) && <p className="error">{(save.error ?? test.error)?.message}</p>}

      <div className="form-actions settings-actions">
        <button type="submit" className="btn btn-primary" disabled={busy || !complete}>
          {save.isPending ? 'Saving…' : 'Save'}
        </button>
        <button
          type="button"
          className="btn btn-secondary"
          disabled={busy || !complete}
          onClick={() => test.mutate(values)}
          title="Tries what's entered above, without saving it"
        >
          <PlugZap size={16} aria-hidden="true" />
          {test.isPending ? 'Testing…' : 'Test connection'}
        </button>
        {saved.configured && (
          <button
            type="button"
            className="btn btn-secondary settings-remove"
            disabled={busy}
            onClick={() => setConfirmRemove(true)}
          >
            Remove
          </button>
        )}
      </div>

      <ConfirmDialog
        open={confirmRemove}
        title="Remove SkipCash credentials"
        message="Online payment turns off: new app bookings will have nothing to pay. Bookings already waiting for payment can no longer be paid in the app."
        confirmLabel="Remove"
        pending={remove.isPending}
        error={remove.error?.message}
        onConfirm={() => remove.mutate(undefined, { onSuccess: () => setConfirmRemove(false) })}
        onClose={() => setConfirmRemove(false)}
      />
    </form>
  )
}
