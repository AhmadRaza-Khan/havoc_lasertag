import type { CSSProperties } from 'react'
import { Medal } from 'lucide-react'
import { statIcons } from '../icons'
import type { PlayerDetails, Tally } from '../types'

type Tile = { label: string; icon: string; value: string; caption: string }

/** The rest of the report: personal-page stats, medals and rivals, over all games. */
export function PersonalStats({ details: d }: { details: PlayerDetails }) {
  const n = (v: number | null) => (v === null ? '—' : String(v))
  const tiles: Tile[] = [
    { label: 'Captures', icon: statIcons.captures, value: n(d.captures), caption: 'Total in all games' },
    { label: 'Hits taken', icon: statIcons.hitsTaken, value: n(d.hits_taken), caption: 'Total in all games' },
    {
      label: 'Efficiency KDR',
      icon: statIcons.efficiency,
      value: d.efficiency_kdr === null ? '—' : d.efficiency_kdr.toFixed(2),
      caption: 'Average per game',
    },
    { label: 'Max combo', icon: statIcons.maxCombo, value: n(d.max_combo), caption: 'Best in one game' },
    {
      label: 'Place in team',
      icon: statIcons.placeInTeam,
      value: d.place_in_team ? ordinal(d.place_in_team.best) : '—',
      caption: d.place_in_team ? `Best · average ${d.place_in_team.average}` : 'Not recorded',
    },
    {
      label: 'Place in game',
      icon: statIcons.rank,
      value: d.place_in_game ? ordinal(d.place_in_game.best) : '—',
      caption: d.place_in_game ? `Best · average ${d.place_in_game.average}` : 'Not recorded',
    },
  ]

  return (
    <>
      <h2 className="ps-section">
        <span>Personal stats</span>
      </h2>
      <section className="ps-details" aria-label="Personal stats">
        {tiles.map((t, i) => (
          <div key={t.label} className="ps-detail" style={{ '--i': i } as CSSProperties}>
            <span className="ps-icon" style={{ '--icon': `url(${t.icon})` } as CSSProperties} aria-hidden="true" />
            <div className="ps-detail-text">
              <span className="ps-bar-label">{t.label}</span>
              <span className="ps-detail-value">{t.value}</span>
              <span className="ps-scale">{t.caption}</span>
            </div>
          </div>
        ))}
      </section>

      <div className="ps-extras">
        <section className="ps-panel" aria-label="Medals">
          <h3 className="ps-panel-title">
            <Medal size={18} aria-hidden="true" />
            Medals
          </h3>
          {d.medals.length === 0 ? (
            <p className="ps-none">No medals yet.</p>
          ) : (
            <ul className="ps-medals">
              {d.medals.map((m) => (
                <li key={m.name} className="ps-medal">
                  <Medal size={16} aria-hidden="true" />
                  <span>{m.name}</span>
                  {m.count > 1 && <span className="ps-medal-count">×{m.count}</span>}
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="ps-panel" aria-label="Rivals">
          <div className="ps-rivals">
            <RivalList title="Most tagged" icon={statIcons.kills} people={d.victims} empty="Nobody tagged yet." />
            <RivalList title="Tagged by" icon={statIcons.kia} people={d.insulters} empty="Nobody yet." />
          </div>
        </section>
      </div>
    </>
  )
}

function RivalList({ title, icon, people, empty }: { title: string; icon: string; people: Tally[]; empty: string }) {
  const max = Math.max(1, ...people.map((p) => p.count))
  return (
    <div>
      <h3 className="ps-panel-title">
        <span className="ps-mini-icon" style={{ '--icon': `url(${icon})` } as CSSProperties} aria-hidden="true" />
        {title}
      </h3>
      {people.length === 0 ? (
        <p className="ps-none">{empty}</p>
      ) : (
        <ol className="ps-rival-list">
          {people.map((p, i) => (
            <li key={p.name} style={{ '--i': i } as CSSProperties}>
              <span className="ps-rival-name">{p.name}</span>
              <span className="ps-rival-bar" aria-hidden="true">
                <span className="ps-fill" style={{ '--fill': `${(p.count / max) * 100}%` } as CSSProperties} />
              </span>
              <span className="ps-rival-count">×{p.count}</span>
            </li>
          ))}
        </ol>
      )}
    </div>
  )
}

/** 1st, 2nd, 3rd, 4th… */
function ordinal(n: number): string {
  const rem100 = n % 100
  if (rem100 >= 11 && rem100 <= 13) return `${n}th`
  return `${n}${({ 1: 'st', 2: 'nd', 3: 'rd' } as Record<number, string>)[n % 10] ?? 'th'}`
}
