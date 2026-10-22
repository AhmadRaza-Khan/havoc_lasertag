import { Link } from 'react-router'
import { LegalLayout } from '../LegalLayout'
import { BUSINESS } from '../business'

/** Public terms of use; players agree to them when they sign up. */
export default function TermsPage() {
  return (
    <LegalLayout title="Terms of use">
      <p className="legal-updated">Effective {BUSINESS.effective}</p>

      <p>
        These terms apply to the Havoc app and related services run by {BUSINESS.name} ("Havoc", "we"). By creating an
        account or using the app you agree to them. Please also read our <Link to="/privacy">privacy policy</Link>.
      </p>

      <h2>Your account</h2>
      <ul>
        <li>Give accurate details, keep your password private, and don't share your account.</li>
        <li>Your playername is public in the app and on our leaderboard; choose one that isn't offensive or misleading.</li>
        <li>You can delete your account at any time in the app (Profile → Delete account) or on our <Link to="/delete-account">account deletion page</Link>.</li>
      </ul>

      <h2>Chat and conduct: zero tolerance</h2>
      <p>
        We have <strong>zero tolerance for objectionable content or abusive behaviour</strong>. In chat, in your
        playername and in your profile photo, you must not:
      </p>
      <ul>
        <li>harass, bully, threaten or intimidate anyone;</li>
        <li>post hateful, discriminatory, sexual, violent or otherwise offensive content;</li>
        <li>spam, advertise, scam or impersonate others;</li>
        <li>share other people's personal information without their permission;</li>
        <li>break the law or encourage others to.</li>
      </ul>
      <p>
        You can block any player and report players or messages from the chat. Our staff review reports, normally
        within 24 hours, and may remove content, warn, suspend or permanently remove accounts that break these rules.
        Offensive words in messages are filtered automatically.
      </p>

      <h2>Bookings and payments</h2>
      <ul>
        <li>Bookings are subject to availability and to the venue's booking, cancellation and safety rules.</li>
        <li>
          Card payments are processed by SkipCash on its secure payment page. Some bookings must be paid within a set time,
          or they are cancelled automatically and the place is released.
        </li>
        <li>For changes, cancellations or refunds, contact us using the details below.</li>
      </ul>

      <h2>Statistics</h2>
      <p>
        Game statistics come from the arena's equipment and reports. We do our best to show them accurately but can't
        guarantee they are error-free, and rankings may change as new games are recorded.
      </p>

      <h2>The service</h2>
      <p>
        We may update, change or stop features of the app. The app is provided "as is"; to the extent the law allows,
        we aren't liable for indirect losses arising from its use. Nothing in these terms limits rights you have under
        the law that can't be limited.
      </p>

      <h2>Ending your use</h2>
      <p>
        You can stop using the app and delete your account at any time. We may suspend or close accounts that break
        these terms or put other players or our staff at risk.
      </p>

      <h2>Law</h2>
      <p>These terms are governed by the laws of the State of Qatar.</p>

      <h2>Contact us</h2>
      <p>
        {BUSINESS.name}, {BUSINESS.address}
        <br />
        Phone: <a href={BUSINESS.phoneHref}>{BUSINESS.phone}</a> · Website: <a href={BUSINESS.website}>{BUSINESS.website}</a>
        <br />
        You can also message <strong>Havoc Admin</strong> in the app's chat.
      </p>
    </LegalLayout>
  )
}
