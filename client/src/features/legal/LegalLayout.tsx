import { useEffect, type ReactNode } from 'react'
import { Link } from 'react-router'
import { BUSINESS } from './business'
import './legal.css'

/** Public page frame for the policies and account deletion (no login). */
export function LegalLayout({ title, children }: { title: string; children: ReactNode }) {
  // Players land here from the app and the store listings: the tab says what the page is, not
  // "Havoc Admin".
  useEffect(() => {
    const previous = document.title
    document.title = `${title} · Havoc`
    return () => {
      document.title = previous
    }
  }, [title])

  return (
    <div className="legal">
      <header className="legal-header">
        <img src="/logo.png" alt={BUSINESS.name} className="legal-logo" />
      </header>
      <main className="legal-main">
        <h1>{title}</h1>
        {children}
      </main>
      <footer className="legal-footer">
        <nav>
          <Link to="/privacy">Privacy policy</Link>
          <Link to="/terms">Terms of use</Link>
          <Link to="/delete-account">Delete your account</Link>
        </nav>
        <p>
          {BUSINESS.name} · {BUSINESS.address} · <a href={BUSINESS.phoneHref}>{BUSINESS.phone}</a>
        </p>
      </footer>
    </div>
  )
}
