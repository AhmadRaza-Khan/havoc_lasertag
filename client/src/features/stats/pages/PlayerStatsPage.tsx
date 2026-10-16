import type { CSSProperties } from 'react'
import { Link, useParams } from 'react-router'
import { ArrowLeft } from 'lucide-react'
import '@fontsource/rajdhani/600.css'
import '@fontsource/rajdhani/700.css'
import { formatDay } from '@/lib/format'
import { useLeaderboard, usePlayerStanding } from '../hooks'
import type { Standing } from '../types'
import { statIcons as icons } from '../icons'
import '../stats.css'

type Bar = {
  label: string
  icon: string
  value: string
  /** 0..1 */
  fill: number
  /** What a full bar means, e.g. "Top 30 best 346". */
  scale: string
}

/** One player's all-time totals, opened from the leaderboard. */
export default function PlayerStatsPage() {
  const { id = '' } = useParams()
  const standing = usePlayerStanding(id)
  const leaderboard = useLeaderboard()
  const s = standing.data

  return (
    <div className="ps">
      <Link to="/" className="ps-back">
        <ArrowLeft size={16} aria-hidden="true" />
        Back to leaderboard
      </Link>

      {standing.isPending ? (
        <p className="muted">Loading stats…</p>
      ) : standing.isError ? (
        <p className="error">{standing.error.message}</p>
      ) : (
        <>
          <header className="ps-hero">
            <div className="ps-crest">
              <span className="ps-wing" aria-hidden="true" />
              <div
                className={`ps-rank${s!.position === 1 ? ' top' : ''}`}
                aria-label={s!.position ? `Rank ${s!.position}` : 'Not ranked yet'}
              >
                <span className="ps-rank-label">Rank</span>
                <span className="ps-rank-value">{s!.position ? `#${s!.position}` : '—'}</span>
              </div>
              <span className="ps-wing right" aria-hidden="true" />
            </div>
            <span className="ps-kicker">Operator</span>
            <h1 className="ps-name">{s!.player.full_name}</h1>
            <p className="ps-meta">
              <span className="ps-handle">@{s!.player.playername}</span>
              <span className="ps-dot" aria-hidden="true" />
              {s!.last_played_at ? `Last played ${formatDay(s!.last_played_at)}` : 'No battles yet'}
            </p>
          </header>

          <section className="ps-bars" aria-label="All-time totals">
            {bars(s!, leaderboard.data ?? []).map((b, i) => (
              <StatBar key={b.label} bar={b} index={i} />
            ))}
          </section>
        </>
      )}
    </div>
  )
}

function StatBar({ bar, index }: { bar: Bar; index: number }) {
  const pct = Math.round(bar.fill * 100)
  return (
    <div className="ps-bar" style={{ '--i': index } as CSSProperties}>
      <span className="ps-icon" style={{ '--icon': `url(${bar.icon})` } as CSSProperties} aria-hidden="true" />
      <span className="ps-bar-value">{bar.value}</span>
      <span className="ps-bar-label">{bar.label}</span>
      <div
        className="ps-track"
        role="meter"
        aria-label={bar.label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
        aria-valuetext={`${bar.value} (${bar.scale})`}
      >
        <span className="ps-fill" style={{ '--fill': `${Math.max(pct, 2)}%` } as CSSProperties} />
      </div>
      <span className="ps-scale">{bar.scale}</span>
    </div>
  )
}

/**
 * The bars, each measured against the best in the top 30 (accuracy against 100%, rank
 * against #1), so a full bar means "as good as the leader".
 */
function bars(s: Standing, board: Standing[]): Bar[] {
  const best = (pick: (x: Standing) => number) => Math.max(pick(s), ...board.map(pick))
  const ratio = (v: number, max: number) => (max > 0 ? Math.min(Math.max(v / max, 0), 1) : 0)
  const relative = (
    label: string,
    icon: string,
    pick: (x: Standing) => number,
    show: (v: number) => string = String,
  ): Bar => {
    const max = best(pick)
    return { label, icon, value: show(pick(s)), fill: ratio(pick(s), max), scale: `Top 30 best ${show(max)}` }
  }
  const ranked = board.filter((x) => x.position !== null).length || 1

  return [
    {
      label: 'Rank',
      icon: icons.rank,
      value: s.position ? `#${s.position}` : '—',
      fill: s.position ? 1 - (s.position - 1) / Math.max(ranked, s.position) : 0,
      scale: s.position ? `Out of ${Math.max(ranked, s.position)} ranked players` : 'Not ranked yet',
    },
    relative('Score', icons.score, (x) => x.points),
    relative('Efficiency', icons.efficiency, (x) => x.efficiency, (v) => v.toFixed(2)),
    relative('Kills', icons.kills, (x) => x.frags),
    relative('KIA', icons.kia, (x) => x.deactivated),
    relative('Shots', icons.shots, (x) => x.shots),
    relative('Hits', icons.hits, (x) => x.hits),
    { label: 'Accuracy', icon: icons.accuracy, value: `${s.accuracy}%`, fill: ratio(s.accuracy, 100), scale: 'Out of 100%' },
  ]
}
