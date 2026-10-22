import { Link } from 'react-router'
import { LegalLayout } from '../LegalLayout'
import { BUSINESS } from '../business'

/** Public privacy policy, linked from the app, the store listings and the website. */
export default function PrivacyPage() {
  return (
    <LegalLayout title="Privacy policy">
      <p className="legal-updated">Effective {BUSINESS.effective}</p>

      <p>
        This policy explains what information the Havoc app and its related services collect, how we use it, who we
        share it with, and the choices you have. "We", "us" and "Havoc" mean {BUSINESS.name}, {BUSINESS.address}.
      </p>

      <h2>Information we collect</h2>
      <ul>
        <li>
          <strong>Account details</strong> you give us when you sign up or when our staff create your account: full
          name, playername, email address and phone number, and your password (stored only in a securely hashed form
          that nobody can read).
        </li>
        <li>
          <strong>Profile photo</strong>, if you choose to add one.
        </li>
        <li>
          <strong>Game statistics</strong> from the arena's game reports: scores, kills, hits, accuracy, placings,
          medals and similar results, and who tagged whom in a game.
        </li>
        <li>
          <strong>Bookings</strong>: the game, date and time, number of players, other players you add, add-ons and the
          price.
        </li>
        <li>
          <strong>Payments</strong>: card payments are made on SkipCash's secure payment page. We never see or store your
          card details; we receive only the payment status, amount and a transaction reference.
        </li>
        <li>
          <strong>Chat messages</strong> you send and receive in the app, and reports you make about other players.
        </li>
        <li>
          <strong>Device and usage information</strong>: a push notification token for your device, when you were last
          active in the app (used for online status), and technical logs such as IP addresses that our servers record
          for security.
        </li>
      </ul>

      <h2>How we use it</h2>
      <ul>
        <li>To create and run your account and let you sign in.</li>
        <li>To show your statistics, rankings and the leaderboard.</li>
        <li>To make, manage and take payment for your bookings, and to send you booking and payment emails.</li>
        <li>To deliver chat messages and send you notifications about new messages.</li>
        <li>To keep players safe: reviewing reported messages and players, and enforcing our terms.</li>
        <li>To provide support, prevent fraud and abuse, and keep our services secure.</li>
      </ul>
      <p>We do not sell your personal information, and we do not use it for advertising.</p>

      <h2>What other people can see</h2>
      <ul>
        <li>
          Other players can see your playername, profile photo, statistics, ranking and whether you're online. They do
          not see your full name, email or phone number.
        </li>
        <li>
          The public leaderboard on our website shows the top players' playernames and statistics.
        </li>
        <li>Chat messages are visible only to you and the player you're chatting with, and to our staff if a message is reported.</li>
        <li>Our staff can see your account details, bookings and statistics to run the venue and help you.</li>
      </ul>

      <h2>Service providers we share information with</h2>
      <p>We use trusted providers to run our services. They process information only to provide their service to us:</p>
      <ul>
        <li>
          <strong>Bookeo</strong> (booking system): your name, contact details and booking details.
        </li>
        <li>
          <strong>SkipCash</strong> (card payments): your name, email, phone number, the payment amount and a booking
          reference; you enter your card details directly on SkipCash's page.
        </li>
        <li>
          <strong>Expo and Google Firebase</strong> (push notifications): your device's notification token and the
          notification content.
        </li>
        <li>
          <strong>SMTP2GO</strong> (email delivery): your email address and the email content.
        </li>
        <li>
          <strong>Our hosting provider</strong>, where our servers and database run.
        </li>
      </ul>
      <p>We may also disclose information if required by law or to protect the safety of our players and staff.</p>

      <h2>How long we keep it</h2>
      <p>
        We keep your information while your account exists. When you delete your account, we delete your account
        details, profile photo, chats, bookings in our app and device tokens straight away. Your past game results stay
        in the game records but are no longer linked to you. Our booking and payment providers keep their own records
        of bookings and payments for as long as the law requires (for example for accounting), and reports about chat
        safety may be kept to protect other players.
      </p>

      <h2>Your choices and rights</h2>
      <ul>
        <li>
          <strong>Delete your account</strong> at any time in the app (Profile → Delete account) or on our{' '}
          <Link to="/delete-account">account deletion page</Link>.
        </li>
        <li>Change or remove your profile photo in the app.</li>
        <li>Turn off notifications in your phone's settings.</li>
        <li>Block or report other players in chat.</li>
        <li>
          Ask us for a copy of your information, or to correct it, using the contact details below.
        </li>
      </ul>

      <h2>Children</h2>
      <p>
        The app is meant for players aged 13 and over. A younger player should have a parent or guardian create and
        manage the account. If you believe a child has given us information without consent, contact us and we will
        delete it.
      </p>

      <h2>Security</h2>
      <p>
        We protect your information with encrypted connections (HTTPS), hashed passwords and restricted access for our
        staff. No system is perfectly secure, but we work to keep your information safe.
      </p>

      <h2>Changes to this policy</h2>
      <p>
        If we change this policy, we'll update the date above and, for important changes, let you know in the app.
      </p>

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
