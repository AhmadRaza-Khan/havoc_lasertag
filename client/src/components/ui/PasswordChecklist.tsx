import { Circle, CircleCheck, CircleX } from 'lucide-react'
import { passwordRules } from '@/lib/password'

/** What a new password still needs, ticked off while typing. */
export function PasswordChecklist({ password }: { password: string }) {
  return (
    <ul className="password-checklist" aria-label="Password requirements">
      {passwordRules(password).map((r) => {
        const Icon = r.ok ? CircleCheck : password ? CircleX : Circle
        return (
          <li key={r.label} className={r.ok ? 'is-ok' : password ? 'is-missing' : undefined}>
            <Icon size={14} aria-hidden="true" />
            {r.label}
            <span className="sr-only">{r.ok ? ' (done)' : ' (not yet)'}</span>
          </li>
        )
      })}
    </ul>
  )
}
