# Store submission: Havoc app

What App Store Connect and Google Play Console ask for, and the answers for this app. The
answers below describe what the app actually does; update them if the app changes.

## Identifiers and links

| Field | Value |
|---|---|
| App name | Havoc |
| Android package / iOS bundle ID | `com.havoc.lasertag` |
| Privacy policy URL | https://havoc-admin.cyberboost.io/privacy |
| Terms of use (EULA) URL | https://havoc-admin.cyberboost.io/terms |
| Account deletion URL (Google Play) | https://havoc-admin.cyberboost.io/delete-account |
| Support / marketing URL | https://www.havoc-tlt.com |
| Support phone | +974 7132 3300 |

Business details used on the legal pages are in `client/src/features/legal/business.ts`.

## Built-in requirements (done)

- **Account deletion in the app**: Profile → Delete account (password confirmation). Also
  without the app at the deletion URL above (Google Play requirement).
- **Privacy policy and terms**: public pages above, linked from Profile and from sign-up; players
  must agree to the terms (including zero tolerance for abusive content) to sign up.
- **User-generated content (chat)** — App Store guideline 1.2 / Google Play UGC policy:
  - report a player or a message (chat ⋮ menu, or long-press a message),
  - block a player (chat ⋮ menu; Profile → Blocked players to unblock),
  - offensive words are masked automatically,
  - staff review reports in the admin panel (Chats → Reports), normally within 24 hours.
- **Permission texts**: camera and photo library (profile picture), notifications. No microphone,
  location, contacts or tracking.
- **Encryption**: only standard HTTPS (`ITSAppUsesNonExemptEncryption = false`), so no export
  compliance documents are needed.

## Reviewer demo account (both stores require one)

Create a normal player account for reviewers (e.g. playername `appreview`) and give its
playername and password in **App Review Information** (Apple) and **App access** (Google). Tell
reviewers: sign in with the playername (not an email); bookings and card payment work but charge
a real card, so they can review up to the payment page without paying.

## Apple: App Privacy ("nutrition label")

Tracking: **No** (no data is used to track users across other companies' apps or websites).

Data collected, all **linked to the user**, none used for tracking:

| Type | Data | Purpose |
|---|---|---|
| Contact info | Name, Email address, Phone number | App functionality |
| User content | Photos (profile picture), Other user content (chat messages, reports) | App functionality |
| Identifiers | User ID (account / playername) | App functionality |
| Purchases | Purchase history (bookings and their payments) | App functionality |
| Usage data | Product interaction (when the app was last open, for online status) | App functionality |
| Other data | Game statistics from the arena | App functionality |

Not collected: location, health, financial info (card details go to SkipCash, never to us), contacts,
browsing history, search history, diagnostics, advertising data.

## Google Play: Data safety

- Data is **encrypted in transit**: Yes.
- Users can **request deletion**: Yes (in the app and at the deletion URL).
- Data **shared** with third parties: No (service providers acting for us don't count as
  sharing).

Data collected (all **required** for the account except the photo; purpose: *App
functionality, Account management*; not used for ads):

| Category | Data types |
|---|---|
| Personal info | Name, Email address, Phone number, User IDs |
| Photos and videos | Photos (profile picture — optional) |
| Messages | Other in-app messages (chat) |
| Financial info | Purchase history (bookings/payments; card numbers are entered on SkipCash's page and not collected by the app) |
| App activity | Other user-generated content (reports), Other actions (last active time) |
| Device or other IDs | Device or other IDs (push notification token) |

## Content rating questionnaires

- Users can **interact / communicate** with each other: **Yes** (private chat), with reporting,
  blocking and moderation.
- Users can **share location**: No.
- **Digital purchases**: No (bookings for a real-world venue are paid by card on SkipCash's page;
  Apple's in-app purchase rules don't apply to physical services).
- Violence: the app shows statistics for a laser tag game (no graphic content).
