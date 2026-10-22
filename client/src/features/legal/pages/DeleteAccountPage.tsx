import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import { apiRequest } from '@/lib/api-client'
import { LegalLayout } from '../LegalLayout'

/** Public account deletion (Google Play requires one that works without the app). */
export default function DeleteAccountPage() {
  const [playername, setPlayername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState(false)

  async function submit(e: FormEvent) {
    e.preventDefault()
    if (!confirmed || busy) return
    setBusy(true)
    setError(null)
    try {
      await apiRequest<void>('/player/auth/delete-account', {
        method: 'POST',
        body: JSON.stringify({ playername: playername.trim(), password }),
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
        You can delete your account here or in the app (Profile → Delete account). Deleting is permanent and happens
        straight away. It removes:
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
      ) : (
        <div className="legal-card">
          <form className="legal-form" onSubmit={submit}>
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
              {busy ? 'Deleting…' : 'Delete my account permanently'}
            </button>
          </form>
        </div>
      )}
    </LegalLayout>
  )
}
