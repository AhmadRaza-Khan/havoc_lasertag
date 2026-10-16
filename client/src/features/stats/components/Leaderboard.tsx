import type { CSSProperties } from 'react'
import { Link } from 'react-router'
import '@fontsource/rajdhani/600.css'
import '@fontsource/rajdhani/700.css'
import { useLeaderboard } from '../hooks'
import { statIcons } from '../icons'
import type { Standing } from '../types'

/** Rows shown: the best ranked players. */
export const LEADERBOARD_SIZE = 30

const COLUMNS: { label: string; icon: string }[] = [
  { label: 'Score', icon: statIcons.score },
  { label: 'Efficiency', icon: statIcons.efficiency },
  { label: 'Kills', icon: statIcons.kills },
  { label: 'KIA', icon: statIcons.kia },
  { label: 'Shots', icon: statIcons.shots },
  { label: 'Hits', icon: statIcons.hits },
  { label: 'Accuracy, %', icon: statIcons.accuracy },
]

/** General report: the top players by all-time score. Click a name for that player's stats. */
export function Leaderboard() {
  const { data, isPending, isError, error } = useLeaderboard()
  // Players who haven't played aren't ranked.
  const top = data?.filter((s) => s.position !== null).slice(0, LEADERBOARD_SIZE)
  const ranked = data?.filter((s) => s.position !== null).length ?? 0
  const bestScore = Math.max(0, ...(top ?? []).map((s) => s.points))

  return (
    <section className="lb">
      <header className="lb-head">
        <span className="lb-kicker">Hall of fame</span>
        <h2 className="lb-title">Player standings</h2>
        <p className="lb-sub">
          Top {LEADERBOARD_SIZE} by all-time score{ranked > LEADERBOARD_SIZE ? ` (of ${ranked} ranked players)` : ''}. Click a
          player for their stats.
        </p>
      </header>

      {isPending ? (
        <p className="lb-state">Loading standings…</p>
      ) : isError ? (
        <p className="lb-state error">{error.message}</p>
      ) : !top || top.length === 0 ? (
        <p className="lb-state">No games recorded yet.</p>
      ) : (
        <>
          <Podium top={top.slice(0, 3)} />

          <div className="table-wrap lb-table-wrap">
            <table className="lb-table">
              <thead>
                <tr>
                  <th className="lb-rank-col">#</th>
                  <th>Player</th>
                  {COLUMNS.map((c) => (
                    <th key={c.label} className="num">
                      <span className="lb-th">
                        <span className="lb-th-icon" style={{ '--icon': `url(${c.icon})` } as CSSProperties} aria-hidden="true" />
                        {c.label}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {top.map((s, i) => (
                  <tr key={s.player.id} style={{ '--i': i } as CSSProperties}>
                    <td className="lb-rank-col">
                      <span className={`lb-hex${s.position! <= 3 ? ` lb-hex-${s.position}` : ''}`}>{s.position}</span>
                    </td>
                    <td>
                      <Link to={`/players/${s.player.id}/stats`} className="stack-cell player-link">
                        <span className="strong">{s.player.full_name}</span>
                        <span className="lb-handle">@{s.player.playername}</span>
                      </Link>
                    </td>
                    <td className="num">
                      <div className="lb-score">
                        <span className="lb-score-value">{s.points}</span>
                        <span className="lb-score-track" aria-hidden="true">
                          <span
                            className="lb-score-fill"
                            style={{ '--fill': `${bestScore > 0 ? Math.max(0, (s.points / bestScore) * 100) : 0}%` } as CSSProperties}
                          />
                        </span>
                      </div>
                    </td>
                    <td className="num">{s.efficiency.toFixed(2)}</td>
                    <td className="num">{s.frags}</td>
                    <td className="num">{s.deactivated}</td>
                    <td className="num">{s.shots}</td>
                    <td className="num">{s.hits}</td>
                    <td className="num">{s.accuracy}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  )
}

/** The top three as podium cards: 2nd, 1st (tallest), 3rd. */
function Podium({ top }: { top: Standing[] }) {
  const order = [top[1], top[0], top[2]].filter((s): s is Standing => !!s)
  return (
    <ol className="lb-podium" aria-label="Top three">
      {order.map((s) => (
        <li key={s.player.id} className={`lb-step lb-step-${s.position}`}>
          <Link to={`/players/${s.player.id}/stats`} className="lb-step-link">
            <span className={`lb-hex lb-hex-big lb-hex-${s.position}`}>#{s.position}</span>
            <span className="lb-step-name">{s.player.full_name}</span>
            <span className="lb-handle">@{s.player.playername}</span>
            <span className="lb-step-score">
              <span className="lb-th-icon" style={{ '--icon': `url(${statIcons.score})` } as CSSProperties} aria-hidden="true" />
              {s.points}
              <small>score</small>
            </span>
          </Link>
        </li>
      ))}
    </ol>
  )
}
