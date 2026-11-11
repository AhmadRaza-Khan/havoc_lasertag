import { useEffect } from 'react'
import { Link } from 'react-router'

/** Any address the panel doesn't know: says so, instead of quietly sending you to the login page. */
export function NotFoundPage() {
  useEffect(() => {
    const previous = document.title
    document.title = 'Page not found · Havoc'
    return () => {
      document.title = previous
    }
  }, [])

  return (
    <main className="not-found">
      <img src="/logo.png" alt="Havoc" className="not-found-logo" />
      <h1>Page not found</h1>
      <p className="muted">There's nothing at this address. It may be old or mistyped.</p>
      <Link to="/" className="btn btn-primary">
        Go to Havoc
      </Link>
    </main>
  )
}
