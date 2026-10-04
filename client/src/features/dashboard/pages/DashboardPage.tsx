import { useSearchParams } from 'react-router'
import { PageHeader } from '@/components/layout/PageHeader'
import { useAuth } from '@/features/auth'
import { Leaderboard, StatsUpload } from '@/features/stats'
import '@/features/stats/stats.css'

const TABS = [
  { id: 'leaderboard', label: 'Leaderboard' },
  { id: 'upload', label: 'Upload' },
] as const
type TabId = (typeof TABS)[number]['id']

export default function DashboardPage() {
  const { user } = useAuth()
  // The active tab lives in the URL (?tab=upload) so it survives reloads and can be linked.
  const [searchParams, setSearchParams] = useSearchParams()
  const tab: TabId = searchParams.get('tab') === 'upload' ? 'upload' : 'leaderboard'

  function select(id: TabId) {
    setSearchParams(id === 'leaderboard' ? {} : { tab: id }, { replace: true })
  }

  return (
    <>
      <PageHeader title="Dashboard" description={`Welcome back, ${user?.name}.`} />

      <div className="tabs" role="tablist" aria-label="Dashboard sections">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            id={`tab-${t.id}`}
            aria-selected={tab === t.id}
            aria-controls={`panel-${t.id}`}
            className={`tab${tab === t.id ? ' active' : ''}`}
            onClick={() => select(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Both panels stay mounted (inactive one hidden) so an upload keeps being tracked —
          and the leaderboard refreshes when it finishes — while the other tab is open. */}
      {TABS.map((t) => (
        <div
          key={t.id}
          className="dashboard"
          role="tabpanel"
          id={`panel-${t.id}`}
          aria-labelledby={`tab-${t.id}`}
          hidden={tab !== t.id}
        >
          {t.id === 'leaderboard' ? <Leaderboard /> : <StatsUpload />}
        </div>
      ))}
    </>
  )
}
