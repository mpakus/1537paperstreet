/** Host labels for the desktop shell. Unknown hosts keep the macOS wording. */

export type DesktopHost = 'mac' | 'windows' | 'linux'

export function hostLabel(platform: string, userAgent: string): DesktopHost {
  const probe = `${platform} ${userAgent}`
  if (/Win/.test(probe)) return 'windows'
  if (/Linux|X11|CrOS/.test(probe) && !/Android/.test(probe)) return 'linux'
  return 'mac'
}

export function currentHost(): DesktopHost {
  if (typeof navigator === 'undefined') return 'mac'
  return hostLabel(navigator.platform ?? '', navigator.userAgent ?? '')
}

export function revealLabel(host: DesktopHost = currentHost()): string {
  if (host === 'windows') return 'Show in Explorer'
  if (host === 'linux') return 'Show in Files'
  return 'Reveal in Finder'
}

export function usesTrafficLights(host: DesktopHost = currentHost()): boolean {
  return host === 'mac'
}
