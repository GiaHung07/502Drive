import { useRef, useCallback } from 'react'

interface ToggleThemeOptions {
  currentTheme: 'light' | 'dark'
  /** Applies the concrete next theme ('light' | 'dark'). */
  setTheme: (theme: 'light' | 'dark') => void
  /**
   * CSS variable holding the page background. Our token stores raw HSL
   * channels (e.g. "225 12% 9%") — the hook wraps it in hsl() automatically;
   * full colors ("#0f1117", "hsl(...)") pass through untouched.
   */
  bgVariable?: string
  fallbackDarkBg?: string
  fallbackLightBg?: string
}

/**
 * Telegram-style bidirectional circular theme reveal.
 *
 * Runs entirely outside React (imperative DOM + rAF) so the animation never
 * re-renders the component tree per frame. Dark→Light expands a circular
 * hole from the click point (clip-path path/evenodd punch-out); Light→Dark
 * contracts the light surface toward the toggle button (clip-path circle).
 * All CSS transitions are frozen during the reveal so underlying components
 * recolor instantly instead of lagging behind the wave. Falls back to an
 * instant swap for prefers-reduced-motion and small viewports.
 */
export function useCircularThemeToggle({
  currentTheme,
  setTheme,
  bgVariable = '--color-bg-base',
  fallbackDarkBg = 'hsl(225 12% 9%)',
  fallbackLightBg = 'hsl(220 20% 97%)',
}: ToggleThemeOptions) {
  const isRunningRef = useRef(false)

  const toggleTheme = useCallback(
    (e?: React.MouseEvent<HTMLElement>) => {
      if (isRunningRef.current) return

      const nextTheme = currentTheme === 'dark' ? 'light' : 'dark'

      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
      const isSmallViewport = window.innerWidth <= 768
      if (prefersReducedMotion || isSmallViewport) {
        setTheme(nextTheme)
        return
      }

      const clickX = e?.clientX ?? window.innerWidth / 2
      const clickY = e?.clientY ?? window.innerHeight / 2

      const btnRect = e?.currentTarget?.getBoundingClientRect()
      const iconX = btnRect ? btnRect.left + btnRect.width / 2 : clickX
      const iconY = btnRect ? btnRect.top + btnRect.height / 2 : clickY

      const expanding = nextTheme === 'light'
      const cx = expanding ? clickX : iconX
      const cy = expanding ? clickY : iconY

      const endRadius = Math.hypot(
        Math.max(cx, window.innerWidth - cx),
        Math.max(cy, window.innerHeight - cy)
      )

      // Read the OLD theme's background before switching. Our token holds HSL
      // channels, not a color — wrap it so the veil paints a real color.
      const rawBg = getComputedStyle(document.documentElement)
        .getPropertyValue(bgVariable)
        .trim()
      const oldBg = rawBg
        ? /^\d/.test(rawBg)
          ? `hsl(${rawBg})`
          : rawBg
        : currentTheme === 'dark'
          ? fallbackDarkBg
          : fallbackLightBg

      // Freeze every transition — including the theme-sweep rules applied via
      // html.theme-transitioning — so nothing recolors late. The injected
      // selector must match the sweep's specificity to out-rank it by order.
      const noTransitionStyle = document.createElement('style')
      noTransitionStyle.textContent =
        '*, *::before, *::after, html.theme-transitioning, html.theme-transitioning *, html.theme-transitioning *::before, html.theme-transitioning *::after { transition: none !important; }'
      document.head.appendChild(noTransitionStyle)

      setTheme(nextTheme)

      const veil = document.createElement('div')
      veil.style.cssText = `position:fixed;inset:0;z-index:999999;pointer-events:none;background:${oldBg};`
      document.body.appendChild(veil)

      const W = window.innerWidth
      const H = window.innerHeight
      const DURATION = expanding ? 420 : 450
      const startTime = performance.now()
      isRunningRef.current = true

      const tick = (now: number) => {
        const elapsed = now - startTime
        let t = Math.min(elapsed / DURATION, 1)

        if (expanding) {
          t = 1 - Math.pow(1 - t, 3)
          const r = Math.max(t * endRadius, 0.5)
          veil.style.clipPath =
            'path(evenodd,"' +
            'M0 0L' + W + ' 0L' + W + ' ' + H + 'L0 ' + H + 'Z' +
            'M' + cx + ' ' + (cy - r) +
            'A' + r + ' ' + r + ' 0 1 1 ' + cx + ' ' + (cy + r) +
            'A' + r + ' ' + r + ' 0 1 1 ' + cx + ' ' + (cy - r) + 'Z")'
        } else {
          t = t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
          const r = t >= 1 ? 0 : (1 - t) * endRadius
          veil.style.clipPath = `circle(${r}px at ${cx}px ${cy}px)`
        }

        if (elapsed < DURATION) {
          requestAnimationFrame(tick)
        } else {
          veil.remove()
          noTransitionStyle.remove()
          isRunningRef.current = false
        }
      }

      requestAnimationFrame(tick)
    },
    [currentTheme, setTheme, bgVariable, fallbackDarkBg, fallbackLightBg]
  )

  return { toggleTheme }
}
