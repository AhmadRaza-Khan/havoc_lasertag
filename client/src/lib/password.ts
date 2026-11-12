/**
 * Password rules, the same as the server's (server/src/shared/validation.rs, which has the final
 * say): at least 8 characters with an uppercase letter, a number and a special character, and
 * not a common password. Checked here as the player types.
 */
export const PASSWORD_MIN = 8

export type PasswordRule = { label: string; ok: boolean }

/** Anything that isn't a letter, a number or a space. */
const isSpecial = (c: string) => !/[\p{L}\p{N}\s]/u.test(c)

/** The checklist shown under a new-password field. */
export function passwordRules(password: string): PasswordRule[] {
  return [
    { label: `At least ${PASSWORD_MIN} characters`, ok: [...password].length >= PASSWORD_MIN },
    { label: 'An uppercase letter', ok: /\p{Lu}/u.test(password) },
    { label: 'A number', ok: /[0-9]/.test(password) },
    { label: 'A special character (like ! @ # ?)', ok: [...password].some(isSpecial) },
    { label: 'Not a common or easy-to-guess password', ok: !!password && !isCommonPassword(password) },
  ]
}

export const passwordOk = (password: string) => passwordRules(password).every((r) => r.ok)

// Keep in step with COMMON_PASSWORDS on the server.
// prettier-ignore
const COMMON = [
  'password', 'passw', 'pass', 'qwerty', 'qwertyuiop', 'qwertz', 'azerty', 'asdf', 'asdfgh',
  'asdfghjkl', 'zxcvbn', 'zxcvbnm', 'qazwsx', 'qweasd', 'abc', 'abcd', 'abcde', 'abcdef',
  'abcdefg', 'abcdefgh', 'letmein', 'welcome', 'admin', 'administrator', 'root', 'login',
  'user', 'guest', 'test', 'testing', 'default', 'changeme', 'secret', 'iloveyou', 'love',
  'lovely', 'hello', 'hi', 'monkey', 'dragon', 'master', 'shadow', 'sunshine', 'princess',
  'superman', 'batman', 'trustno', 'freedom', 'whatever', 'starwars', 'football', 'baseball',
  'soccer', 'hockey', 'basketball', 'liverpool', 'chelsea', 'arsenal', 'barcelona',
  'realmadrid', 'manchester', 'ronaldo', 'cristiano', 'messi', 'michael', 'jordan', 'summer',
  'winter', 'spring', 'autumn', 'family', 'computer', 'internet', 'google', 'samsung',
  'apple', 'iphone', 'android', 'mobile', 'money', 'flower', 'angel', 'baby', 'friend',
  'friends', 'killer', 'ninja', 'pokemon', 'minecraft', 'fortnite', 'gaming', 'gamer',
  'player', 'havoc', 'havoctlt', 'lasertag', 'laser', 'tactical', 'qatar', 'doha', 'allah',
  'mohammed', 'muhammad', 'mohamed', 'ahmed', 'ahmad', 'ali', 'omar', 'fatima', 'aaa',
]

const LEET: Record<string, string> = {
  '0': 'o',
  '1': 'i',
  '3': 'e',
  '4': 'a',
  '5': 's',
  '7': 't',
  '@': 'a',
  $: 's',
}

/** A common word dressed up with capitals, digits and symbols: Password1!, P@ssw0rd, Laser@Tag1. */
export function isCommonPassword(password: string): boolean {
  const core = password.replace(/^[^\p{L}]+|[^\p{L}]+$/gu, '').toLowerCase()
  const letters = (s: string) => [...s].filter((c) => /\p{L}/u.test(c)).join('')
  const plain = letters(core)
  const decoded = letters([...core].map((c) => LEET[c] ?? c).join(''))
  return [plain, decoded].some(
    (word) =>
      !word ||
      [...word].every((c) => c === word[0]) ||
      COMMON.some((common) => word === common || (common.length >= 6 && word.includes(common))),
  )
}
