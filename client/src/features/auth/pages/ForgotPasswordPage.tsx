import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { PasswordChecklist } from '@/components/ui/PasswordChecklist'
import { passwordOk } from '@/lib/password'
import { authApi } from '../api'

/**
 * Staff "forgot password": a 6-digit code goes to the email on their account (set in Settings).
 * Accounts without an email are reset by the owner.
 */
export default function ForgotPasswordPage() {
  const navigate = useNavigate()
  const [step, setStep] = useState<'ask' | 'reset'>('ask')
  const [login, setLogin] = useState('')
  const [code, setCode] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  async function sendCode(e?: FormEvent) {
    e?.preventDefault()
    if (!login.trim() || busy) return
    setError(null)
    setBusy(true)
    try {
      await authApi.forgotPassword(login.trim())
      setStep('reset')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not send the code')
    } finally {
      setBusy(false)
    }
  }

  async function reset(e: FormEvent) {
    e.preventDefault()
    if (password !== confirm) {
      setError("The passwords don't match")
      return
    }
    setError(null)
    setBusy(true)
    try {
      await authApi.resetPassword(login.trim(), code.trim(), password)
      navigate('/login', { replace: true, state: { reset: true } })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not change the password')
      setBusy(false)
    }
  }

  return (
    <div className="center">
      {step === 'ask' ? (
        <form className="card" onSubmit={sendCode}>
          <div className="login-brand">
            <img src="/logo.png" alt="Havoc Tactical Laser Tag" />
          </div>
          <div>
            <h1 className="login-title">Forgot password</h1>
            <p className="muted">
              Enter your username or the email on your account. We’ll email you a code to choose a
              new password. No email on your account? Ask the owner to reset your password.
            </p>
          </div>
          <label>
            Username or email
            <input
              autoComplete="username"
              autoCapitalize="none"
              spellCheck={false}
              required
              value={login}
              onChange={(e) => setLogin(e.target.value)}
            />
          </label>
          {error && <p className="error">{error}</p>}
          <button type="submit" className="btn btn-primary" disabled={busy}>
            {busy ? 'Sending…' : 'Send code'}
          </button>
          <Link to="/login" className="login-forgot">
            Back to sign in
          </Link>
        </form>
      ) : (
        <form className="card" onSubmit={reset}>
          <div>
            <h1 className="login-title">New password</h1>
            <p className="muted">
              If that account has an email, we’ve sent it a 6-digit code. It works for 15 minutes.
            </p>
          </div>
          <label>
            6-digit code
            <input
              inputMode="numeric"
              autoComplete="one-time-code"
              required
              maxLength={6}
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, '').slice(0, 6))}
            />
          </label>
          <label>
            New password
            <input
              type="password"
              autoComplete="new-password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </label>
          <PasswordChecklist password={password} />
          <label>
            Confirm new password
            <input
              type="password"
              autoComplete="new-password"
              required
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
            />
          </label>
          {error && <p className="error">{error}</p>}
          <button type="submit" className="btn btn-primary" disabled={busy || code.length !== 6 || !passwordOk(password)}>
            {busy ? 'Saving…' : 'Change password'}
          </button>
          <button type="button" className="link-button login-forgot" disabled={busy} onClick={() => void sendCode()}>
            Send a new code
          </button>
        </form>
      )}
    </div>
  )
}
