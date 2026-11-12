import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import { apiRequest } from '@/lib/api-client'
import { LegalLayout } from '../LegalLayout'

/**
 * Public account deletion (Google Play requires one that works without the app). Two steps: the
 * playername and password, then a code emailed to the account's confirmed address.
 */
export default function DeleteAccountPage() {
  const [playername, setPlayername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [sentTo, setSentTo] = useState<string | null>(null)
  const [code, setCode] = useState('')
  const [done, setDone] = useState(false)

  async function requestCode(e?: FormEvent) {
    e?.preventDefault()
    if (!confirmed || busy) return
    setBusy(true)
    setError(null)
    try {
      const res = await apiRequest<{ email_hint: string }>('/player/auth/delete-account', {
        method: 'POST',
        body: JSON.stringify({ playername: playername.trim(), password }),
      })
      setSentTo(res.email_hint)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not continue. Please try again.')
    } finally {
      setBusy(false)
    }
  }

  async function confirmDeletion(e: FormEvent) {
    e.preventDefault()
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      await apiRequest<void>('/player/auth/delete-account/confirm', {
        method: 'POST',
        body: JSON.stringify({ playername: playername.trim(), code: code.trim() }),
      })
      setDone(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not delete the account. Please try again.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <LegalLayout title="Delete your Havoc account">
      <p>
        You can delete your account here or in the app (Profile → Delete account). To delete it here, we email a code to
        your account’s confirmed email address. Deleting is permanent. It removes:
      </p>
      <ul>
        <li>your account details (name, playername, email, phone and password)</li>
        <li>your profile photo</li>
        <li>your chats</li>
        <li>your bookings in the app, and your devices' notification tokens</li>
      </ul>
      <p>
        Your past game results stay in the game records but are no longer linked to you. Our booking and payment
        providers keep their own records as the law requires. A booking you already made at the venue isn't cancelled
        by this, so contact us first if you want to cancel one. See the <Link to="/privacy">privacy policy</Link> for
        details.
      </p>

      {done ? (
        <div className="legal-done" role="status">
          Your account has been deleted. Thank you for playing at Havoc.
        </div>
      ) : sentTo ? (
        <div className="legal-card">
          <form className="legal-form" onSubmit={confirmDeletion}>
            <p>
              We’ve emailed a 6-digit code to <strong>{sentTo}</strong>. It works for 15 minutes.
            </p>
            <label>
              Code from the email
              <input
                inputMode="numeric"
                autoComplete="one-time-code"
                required
                maxLength={6}
                value={code}
                onChange={(e) => setCode(e.target.value.replace(/\D/g, '').slice(0, 6))}
              />
            </label>
            {error && (
              <p className="legal-error" role="alert">
                {error}
              </p>
            )}
            <button type="submit" className="legal-danger" disabled={busy || code.length !== 6}>
              {busy ? 'Deleting…' : 'Delete my account permanently'}
            </button>
            <button type="button" className="legal-link-button" disabled={busy} onClick={() => void requestCode()}>
              Send a new code
            </button>
          </form>
        </div>
      ) : (
        <div className="legal-card">
          <form className="legal-form" onSubmit={requestCode}>
            <label>
              Playername
              <input
                type="text"
                autoComplete="username"
                autoCapitalize="none"
                required
                value={playername}
                onChange={(e) => setPlayername(e.target.value)}
              />
            </label>
            <label>
              Password
              <input
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <span className="legal-hint">
                Forgot it? In the Havoc app, tap <strong>Forgot password?</strong> on the sign-in screen to get a reset code by
                email.
              </span>
            </label>
            <label className="legal-check">
              <input type="checkbox" checked={confirmed} onChange={(e) => setConfirmed(e.target.checked)} />I understand
              that my account and its data will be permanently deleted.
            </label>
            {error && (
              <p className="legal-error" role="alert">
                {error}
              </p>
            )}
            <button type="submit" className="legal-danger" disabled={!confirmed || busy || !playername.trim() || !password}>
              {busy ? 'Checking…' : 'Continue'}
            </button>
          </form>
        </div>
      )}
    </LegalLayout>
  )
}
