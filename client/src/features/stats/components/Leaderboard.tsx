import { useLeaderboard } from '../hooks'

/** General report: every player, ranked by total points (same columns as the PDF summary). */
export function Leaderboard() {
  const { data, isPending, isError, error } = useLeaderboard()

  return (
    <section className="panel">
      <div className="panel-toolbar">
        <div>
          <h2>Player standings</h2>
          <p className="muted small">All games combined, ranked by points.</p>
        </div>
      </div>

      {isPending ? (
        <p className="panel-empty muted">Loading standings…</p>
      ) : isError ? (
        <p className="panel-empty error">{error.message}</p>
      ) : data.length === 0 ? (
        <p className="panel-empty muted">No players yet.</p>
      ) : (
        <div className="table-wrap">
          <table className="table leaderboard">
            <thead>
              <tr>
                <th className="num">#</th>
                <th>Player</th>
                <th className="num">Games</th>
                <th className="num">Points</th>
                <th className="num">Efficiency</th>
                <th className="num">Frags</th>
                <th className="num">Deactivated</th>
                <th className="num">Shots</th>
                <th className="num">Hits</th>
                <th className="num">Accuracy, %</th>
              </tr>
            </thead>
            <tbody>
              {data.map((s) => (
                <tr key={s.player.id} className={s.games === 0 ? 'row-muted' : undefined}>
                  <td className="num">
                    {s.position === null ? (
                      <span className="muted" title="No games yet">
                        —
                      </span>
                    ) : (
                      <span className={`rank${s.position <= 3 ? ` rank-${s.position}` : ''}`}>
                        {s.position}
                      </span>
                    )}
                  </td>
                  <td>
                    <div className="stack-cell">
                      <span className="strong">{s.player.full_name}</span>
                      <span className="muted">@{s.player.playername}</span>
                    </div>
                  </td>
                  <td className="num">{s.games}</td>
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
