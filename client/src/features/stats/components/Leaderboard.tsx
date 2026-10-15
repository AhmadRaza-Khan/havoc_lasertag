import { Link } from 'react-router'
import { useLeaderboard } from '../hooks'

/** Rows shown: the best ranked players. */
export const LEADERBOARD_SIZE = 30

/** General report: the top players by all-time score. Click a name for that player's stats. */
export function Leaderboard() {
  const { data, isPending, isError, error } = useLeaderboard()
  // Players who haven't played aren't ranked.
  const top = data?.filter((s) => s.position !== null).slice(0, LEADERBOARD_SIZE)
  const ranked = data?.filter((s) => s.position !== null).length ?? 0

  return (
    <section className="panel">
      <div className="panel-toolbar">
        <div>
          <h2>Player standings</h2>
          <p className="muted small">
            Top {LEADERBOARD_SIZE} by all-time score{ranked > LEADERBOARD_SIZE ? ` (of ${ranked} ranked players)` : ''}. Click a
            player for their stats.
          </p>
        </div>
      </div>

      {isPending ? (
        <p className="panel-empty muted">Loading standings…</p>
      ) : isError ? (
        <p className="panel-empty error">{error.message}</p>
      ) : !top || top.length === 0 ? (
        <p className="panel-empty muted">No games recorded yet.</p>
      ) : (
        <div className="table-wrap">
          <table className="table leaderboard">
            <thead>
              <tr>
                <th className="num">#</th>
                <th>Player</th>
                <th className="num">Score</th>
                <th className="num">Efficiency</th>
                <th className="num">Kills</th>
                <th className="num">KIA</th>
                <th className="num">Shots</th>
                <th className="num">Hits</th>
                <th className="num">Accuracy, %</th>
              </tr>
            </thead>
            <tbody>
              {top.map((s) => (
                <tr key={s.player.id}>
                  <td className="num">
                    <span className={`rank${s.position! <= 3 ? ` rank-${s.position}` : ''}`}>{s.position}</span>
                  </td>
                  <td>
                    <Link to={`/players/${s.player.id}/stats`} className="stack-cell player-link">
                      <span className="strong">{s.player.full_name}</span>
                      <span className="muted">@{s.player.playername}</span>
                    </Link>
                  </td>
                  <td className="num strong">{s.points}</td>
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
      )}
    </section>
  )
}
