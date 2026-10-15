import { Fragment, useState } from 'react'
import { Link, useParams } from 'react-router'
import { ArrowLeft, ChevronDown, ChevronRight } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { Pagination } from '@/components/ui/Pagination'
import { formatDay, formatTimeRange } from '@/lib/format'
import { usePlayerGames, usePlayerStanding } from '../hooks'
import type { GameRecord } from '../types'
import '../stats.css'

/** All of one player's stats: all-time totals and every game they played. */
export default function PlayerStatsPage() {
  const { id = '' } = useParams()
  const [page, setPage] = useState(1)
  const standing = usePlayerStanding(id)
  const games = usePlayerGames(id, page)
  const s = standing.data

  return (
    <>
      <PageHeader
        title={s ? s.player.full_name : 'Player stats'}
        description={s ? `@${s.player.playername}${s.position ? ` · Rank #${s.position}` : ' · Not ranked yet'}` : undefined}
        actions={
          <Link to="/" className="btn btn-secondary">
            <ArrowLeft size={16} aria-hidden="true" />
            Back to leaderboard
          </Link>
        }
      />

      {standing.isPending ? (
        <p className="muted">Loading stats…</p>
      ) : standing.isError ? (
        <p className="error">{standing.error.message}</p>
      ) : (
        <div className="player-stats">
          <section className="stat-cards" aria-label="All-time totals">
            <StatCard label="Rank" value={s!.position ? `#${s!.position}` : '—'} />
            <StatCard label="Score" value={s!.points} accent />
            <StatCard label="Efficiency" value={s!.efficiency.toFixed(2)} />
            <StatCard label="Kills" value={s!.frags} />
            <StatCard label="KIA" value={s!.deactivated} />
            <StatCard label="Shots" value={s!.shots} />
            <StatCard label="Hits" value={s!.hits} />
            <StatCard label="Accuracy" value={`${s!.accuracy}%`} />
          </section>

          <section className="panel">
            <div className="panel-toolbar">
              <div>
                <h2>Game history</h2>
                <p className="muted small">
                  {s!.last_played_at ? `Last played ${formatDay(s!.last_played_at)}. ` : ''}Click a game for medals and who
                  they tagged.
                </p>
              </div>
            </div>
            {games.isPending ? (
              <p className="panel-empty muted">Loading games…</p>
            ) : games.isError ? (
              <p className="panel-empty error">{games.error.message}</p>
            ) : games.data.items.length === 0 ? (
              <p className="panel-empty muted">No games recorded for this player yet.</p>
            ) : (
              <>
                <GamesTable games={games.data.items} />
                <Pagination
                  page={games.data.page}
                  perPage={games.data.per_page}
                  total={games.data.total}
                  onPageChange={setPage}
                />
              </>
            )}
          </section>
        </div>
      )}
    </>
  )
}

function StatCard({ label, value, accent }: { label: string; value: string | number; accent?: boolean }) {
  return (
    <div className="stat-card">
      <span className={`stat-card-value${accent ? ' accent' : ''}`}>{value}</span>
      <span className="stat-card-label">{label}</span>
    </div>
  )
}

function GamesTable({ games }: { games: GameRecord[] }) {
  const [open, setOpen] = useState<string | null>(null)
  return (
    <div className="table-wrap">
      <table className="table games-table">
        <thead>
          <tr>
            <th aria-label="Details" />
            <th>Date &amp; time</th>
            <th>Scenario</th>
            <th>Team</th>
            <th className="num">Place</th>
            <th className="num">Score</th>
            <th className="num">Kills</th>
            <th className="num">KIA</th>
            <th className="num">Accuracy, %</th>
          </tr>
        </thead>
        <tbody>
          {games.map((g) => {
            const expanded = open === g.game.id
            return (
              <Fragment key={g.game.id}>
                <tr className="clickable-row" onClick={() => setOpen(expanded ? null : g.game.id)}>
                  <td>
                    <button
                      type="button"
                      className="icon-btn"
                      aria-expanded={expanded}
                      aria-label={expanded ? 'Hide game details' : 'Show game details'}
                      onClick={(e) => {
                        e.stopPropagation()
                        setOpen(expanded ? null : g.game.id)
                      }}>
                      {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </button>
                  </td>
                  <td>
                    <div className="stack-cell">
                      <span>{formatDay(g.game.started_at)}</span>
                      <span className="muted">{formatTimeRange(g.game.started_at, g.game.ended_at)}</span>
                    </div>
                  </td>
                  <td>{g.game.scenario ?? <span className="muted">—</span>}</td>
                  <td>{g.team ?? <span className="muted">—</span>}</td>
                  <td className="num">
                    {g.place_in_game ?? '—'}
                    {g.place_in_team !== null && <span className="muted small"> ({g.place_in_team} in team)</span>}
                  </td>
                  <td className="num strong">{g.points}</td>
                  <td className="num">{g.frags}</td>
                  <td className="num">{g.deactivated}</td>
                  <td className="num">{g.accuracy}</td>
                </tr>
                {expanded && (
                  <tr className="details-row">
                    <td />
                    <td colSpan={8}>
                      <GameDetails g={g} />
                    </td>
                  </tr>
                )}
              </Fragment>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function GameDetails({ g }: { g: GameRecord }) {
  const extras: [string, string | number | null][] = [
    ['Efficiency', g.efficiency.toFixed(2)],
    ['KDR', g.efficiency_kdr?.toFixed(2) ?? null],
    ['Shots', g.shots],
    ['Hits', g.hits],
    ['Hits taken', g.hits_taken],
    ['Max combo', g.max_combo],
    ['Captures', g.captures],
    ['Team score', g.team_score],
    ['Player # in game', g.number_in_game],
  ]
  return (
    <div className="game-details">
      <dl className="game-extras">
        {extras
          .filter(([, v]) => v !== null)
          .map(([k, v]) => (
            <div key={k}>
              <dt>{k}</dt>
              <dd>{v}</dd>
            </div>
          ))}
      </dl>
      <div className="game-lists">
        <div>
          <h3>Medals</h3>
          {g.nominations.length ? (
            <div className="medals">
              {g.nominations.map((n) => (
                <span key={n} className="badge badge-medal">
                  {n}
                </span>
              ))}
            </div>
          ) : (
            <p className="muted small">None</p>
          )}
        </div>
        <NamedList title="Tagged" people={g.victims} />
        <NamedList title="Tagged by" people={g.insulters} />
      </div>
    </div>
  )
}

function NamedList({ title, people }: { title: string; people: { name: string; count: number | null }[] }) {
  return (
    <div>
      <h3>{title}</h3>
      {people.length ? (
        <ul className="named-list">
          {people.map((p, i) => (
            <li key={`${p.name}-${i}`}>
              {p.name}
              {p.count !== null && <span className="muted"> ×{p.count}</span>}
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted small">Nobody</p>
      )}
    </div>
  )
}
