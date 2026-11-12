import { useState } from 'react'
import type { FormEvent } from 'react'
import { CircleCheck, KeyRound, Pencil, Trash2, UserPlus } from 'lucide-react'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { Modal } from '@/components/ui/Modal'
import { useAuth } from '@/features/auth'
import {
  useChangeMyPassword,
  useCreateStaff,
  useRemoveStaff,
  useSetStaffPassword,
  useStaff,
  useUpdateMe,
  useUpdateStaff,
} from '../hooks'
import type { StaffUser } from '../types'

/**
 * Your own account (name, email for password resets, password) and, for the owner, every staff
 * account: add, edit, set a password by hand, remove.
 */
export function StaffSettings() {
  const { user } = useAuth()
  if (!user) return null
  return (
    <>
      <MyAccount />
      {user.is_owner && <StaffAccounts />}
    </>
  )
}

function MyAccount() {
  const { user, updateUser } = useAuth()
  const [name, setName] = useState(user?.name ?? '')
  const [email, setEmail] = useState(user?.email ?? '')
  const [saved, setSaved] = useState(false)
  const update = useUpdateMe()

  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirm, setConfirm] = useState('')
  const [changed, setChanged] = useState(false)
  const change = useChangeMyPassword()
  const mismatch = confirm !== '' && confirm !== next

  function saveProfile(e: FormEvent) {
    e.preventDefault()
    setSaved(false)
    update.mutate(
      { name, email },
      {
        onSuccess: (u) => {
          updateUser(u)
          setSaved(true)
        },
      },
    )
  }

  function changePassword(e: FormEvent) {
    e.preventDefault()
    if (mismatch) return
    setChanged(false)
    change.mutate(
      { current, next },
      {
        onSuccess: () => {
          setCurrent('')
          setNext('')
          setConfirm('')
          setChanged(true)
        },
      },
    )
  }

  return (
    <section className="settings-section">
      <h2>Your account</h2>
      <p className="muted">
        Signed in as <strong>@{user?.username}</strong>
        {user?.is_owner ? ' (owner)' : ''}. Add your email so you can reset a forgotten password from the
        sign-in page.
      </p>

      <form className="form" onSubmit={saveProfile}>
        <div className="form-row">
          <label>
            Name
            <input required value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            Email
            <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} placeholder="Not set" />
          </label>
        </div>
        {update.error && <p className="error">{update.error.message}</p>}
        <div className="form-actions settings-actions">
          {saved && !update.isPending && (
            <span className="notice notice-success saved-note" role="status">
              <CircleCheck size={16} aria-hidden="true" />
              Saved.
            </span>
          )}
          <button type="submit" className="btn btn-primary" disabled={update.isPending}>
            {update.isPending ? 'Saving…' : 'Save'}
          </button>
        </div>
      </form>

      <form className="form" onSubmit={changePassword}>
        <h3 className="settings-subtitle">Change password</h3>
        <label>
          Current password
          <input
            type="password"
            autoComplete="current-password"
            required
            value={current}
            onChange={(e) => setCurrent(e.target.value)}
          />
        </label>
        <div className="form-row">
          <label>
            New password
            <input
              type="password"
              autoComplete="new-password"
              required
              minLength={8}
              value={next}
              onChange={(e) => setNext(e.target.value)}
            />
          </label>
          <label>
            Confirm new password
            <input
              type="password"
              autoComplete="new-password"
              required
              minLength={8}
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              aria-invalid={mismatch}
            />
          </label>
        </div>
        {mismatch && <p className="error">The passwords don’t match.</p>}
        {change.error && <p className="error">{change.error.message}</p>}
        <div className="form-actions settings-actions">
          {changed && !change.isPending && (
            <span className="notice notice-success saved-note" role="status">
              <CircleCheck size={16} aria-hidden="true" />
              Password changed. Your other sessions were signed out.
            </span>
          )}
          <button type="submit" className="btn btn-secondary" disabled={change.isPending || mismatch}>
            <KeyRound size={16} aria-hidden="true" />
            {change.isPending ? 'Changing…' : 'Change password'}
          </button>
        </div>
      </form>
    </section>
  )
}

type Dialog =
  | { kind: 'add' }
  | { kind: 'edit'; user: StaffUser }
  | { kind: 'password'; user: StaffUser }
  | { kind: 'remove'; user: StaffUser }
  | null

function StaffAccounts() {
  const { user: me } = useAuth()
  const { data, isPending, isError, error } = useStaff()
  const [dialog, setDialog] = useState<Dialog>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const remove = useRemoveStaff()

  return (
    <section className="settings-section staff-section">
      <div className="settings-heading">
        <h2>Staff accounts</h2>
        <button type="button" className="btn btn-secondary btn-sm" onClick={() => setDialog({ kind: 'add' })}>
          <UserPlus size={14} aria-hidden="true" />
          Add staff
        </button>
      </div>
      <p className="muted">
        As the owner you can add staff, change their details, set a new password for anyone who is locked out,
        and remove accounts.
      </p>
      {notice && (
        <p className="notice notice-success" role="status">
          <CircleCheck size={16} aria-hidden="true" />
          {notice}
        </p>
      )}

      {isPending ? (
        <p className="muted">Loading staff…</p>
      ) : isError ? (
        <p className="error">{error.message}</p>
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Email</th>
                <th className="table-actions">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {data.map((u) => (
                <tr key={u.id}>
                  <td>
                    <div className="stack-cell">
                      <span className="strong">
                        {u.name}
                        {u.is_owner && <span className="badge badge-owner">Owner</span>}
                      </span>
                      <span className="muted">@{u.username}</span>
                    </div>
                  </td>
                  <td>{u.email ?? <span className="muted">Not set</span>}</td>
                  <td className="table-actions">
                    <button
                      type="button"
                      className="icon-btn"
                      title="Edit"
                      aria-label={`Edit ${u.username}`}
                      onClick={() => setDialog({ kind: 'edit', user: u })}
                    >
                      <Pencil size={16} />
                    </button>
                    <button
                      type="button"
                      className="icon-btn"
                      title="Set password"
                      aria-label={`Set a new password for ${u.username}`}
                      onClick={() => setDialog({ kind: 'password', user: u })}
                    >
                      <KeyRound size={16} />
                    </button>
                    {!u.is_owner && u.id !== me?.id ? (
                      <button
                        type="button"
                        className="icon-btn icon-btn-danger"
                        title="Remove"
                        aria-label={`Remove ${u.username}`}
                        onClick={() => setDialog({ kind: 'remove', user: u })}
                      >
                        <Trash2 size={16} />
                      </button>
                    ) : (
                      <span className="icon-btn-placeholder" />
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {dialog?.kind === 'add' && (
        <AddStaffDialog
          onClose={() => setDialog(null)}
          onDone={(u) => {
            setDialog(null)
            setNotice(`Staff account @${u.username} added. Give them their password to sign in.`)
          }}
        />
      )}
      {dialog?.kind === 'edit' && (
        <EditStaffDialog
          user={dialog.user}
          onClose={() => setDialog(null)}
          onDone={() => {
            setDialog(null)
            setNotice('Saved.')
          }}
        />
      )}
      {dialog?.kind === 'password' && (
        <SetPasswordDialog
          user={dialog.user}
          onClose={() => setDialog(null)}
          onDone={() => {
            setNotice(
              `New password set for @${dialog.user.username}. Their sessions were signed out; tell them the new password.`,
            )
            setDialog(null)
          }}
        />
      )}
      <ConfirmDialog
        open={dialog?.kind === 'remove'}
        title="Remove staff account"
        message={
          dialog?.kind === 'remove' ? (
            <>
              Remove <strong>@{dialog.user.username}</strong>? They won’t be able to sign in any more.
            </>
          ) : (
            ''
          )
        }
        confirmLabel="Remove"
        pending={remove.isPending}
        error={remove.error?.message}
        onConfirm={() =>
          dialog?.kind === 'remove' &&
          remove.mutate(dialog.user.id, {
            onSuccess: () => {
              setNotice(`@${dialog.user.username} removed.`)
              setDialog(null)
            },
          })
        }
        onClose={() => {
          setDialog(null)
          remove.reset()
        }}
      />
    </section>
  )
}

function AddStaffDialog({ onClose, onDone }: { onClose: () => void; onDone: (u: StaffUser) => void }) {
  const [values, setValues] = useState({ username: '', name: '', email: '', password: '' })
  const create = useCreateStaff()
  const set = (k: keyof typeof values) => (e: { target: { value: string } }) =>
    setValues((v) => ({ ...v, [k]: e.target.value }))
  return (
    <Modal
      open
      title="Add staff"
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose} disabled={create.isPending}>
            Cancel
          </button>
          <button type="submit" form="add-staff" className="btn btn-primary" disabled={create.isPending}>
            {create.isPending ? 'Adding…' : 'Add staff'}
          </button>
        </>
      }
    >
      <form
        id="add-staff"
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          create.mutate(values, { onSuccess: onDone })
        }}
      >
        <div className="form-row">
          <label>
            Name
            <input required value={values.name} onChange={set('name')} />
          </label>
          <label>
            Username
            <input required autoCapitalize="none" spellCheck={false} value={values.username} onChange={set('username')} />
          </label>
        </div>
        <label>
          Email <span className="muted">(for resetting a forgotten password)</span>
          <input type="email" value={values.email} onChange={set('email')} />
        </label>
        <label>
          Password <span className="muted">(at least 8 characters; give it to them)</span>
          <input type="text" required minLength={8} autoComplete="off" value={values.password} onChange={set('password')} />
        </label>
        {create.error && <p className="error">{create.error.message}</p>}
      </form>
    </Modal>
  )
}

function EditStaffDialog({ user, onClose, onDone }: { user: StaffUser; onClose: () => void; onDone: () => void }) {
  const { user: me, updateUser } = useAuth()
  const [name, setName] = useState(user.name)
  const [email, setEmail] = useState(user.email ?? '')
  const update = useUpdateStaff()
  return (
    <Modal
      open
      title={`Edit @${user.username}`}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose} disabled={update.isPending}>
            Cancel
          </button>
          <button type="submit" form="edit-staff" className="btn btn-primary" disabled={update.isPending}>
            {update.isPending ? 'Saving…' : 'Save'}
          </button>
        </>
      }
    >
      <form
        id="edit-staff"
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          update.mutate(
            { id: user.id, input: { name, email } },
            {
              onSuccess: (u) => {
                if (u.id === me?.id) updateUser(u)
                onDone()
              },
            },
          )
        }}
      >
        <label>
          Name
          <input required value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label>
          Email
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} placeholder="Not set" />
        </label>
        {update.error && <p className="error">{update.error.message}</p>}
      </form>
    </Modal>
  )
}

function SetPasswordDialog({ user, onClose, onDone }: { user: StaffUser; onClose: () => void; onDone: () => void }) {
  const [password, setPassword] = useState('')
  const setPw = useSetStaffPassword()
  return (
    <Modal
      open
      title={`New password for @${user.username}`}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onClose} disabled={setPw.isPending}>
            Cancel
          </button>
          <button type="submit" form="set-password" className="btn btn-primary" disabled={setPw.isPending}>
            {setPw.isPending ? 'Saving…' : 'Set password'}
          </button>
        </>
      }
    >
      <form
        id="set-password"
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          setPw.mutate({ id: user.id, password }, { onSuccess: onDone })
        }}
      >
        <p className="muted">
          Their current sessions will be signed out. Give them the new password; they can change it in Settings.
        </p>
        <label>
          New password <span className="muted">(at least 8 characters)</span>
          <input type="text" required minLength={8} autoComplete="off" value={password} onChange={(e) => setPassword(e.target.value)} />
        </label>
        {setPw.error && <p className="error">{setPw.error.message}</p>}
      </form>
    </Modal>
  )
}
