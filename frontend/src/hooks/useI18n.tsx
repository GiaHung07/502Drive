import React, { createContext, useContext, useState, useEffect, useCallback } from 'react'
import { translations, Language } from '@/lib/i18n'

interface I18nContextType {
  lang: Language
  setLang: (lang: Language) => void
  t: (key: string, fallback?: string) => string
}

const I18nContext = createContext<I18nContextType | undefined>(undefined)

const STORAGE_KEY = '502drive-language'

export const I18nProvider: React.FC<{
  children: React.ReactNode
  initialLang?: Language
  onLangChange?: (lang: Language) => void
}> = ({ children, initialLang = 'vi', onLangChange }) => {
  const [lang, setLangState] = useState<Language>(() => {
    const saved = localStorage.getItem(STORAGE_KEY) as Language | null
    if (saved === 'vi' || saved === 'en') return saved
    return initialLang
  })

  // Synchronize when initialLang changes from external config (e.g. AppConfig load)
  useEffect(() => {
    if (initialLang && (initialLang === 'vi' || initialLang === 'en')) {
      setLangState(initialLang)
      localStorage.setItem(STORAGE_KEY, initialLang)
    }
  }, [initialLang])

  const setLang = useCallback(
    (newLang: Language) => {
      setLangState(newLang)
      localStorage.setItem(STORAGE_KEY, newLang)
      onLangChange?.(newLang)
    },
    [onLangChange]
  )

  const t = useCallback(
    (path: string, fallback?: string): string => {
      const parts = path.split('.')
      let current: unknown = translations[lang]

      for (const part of parts) {
        if (current && typeof current === 'object' && part in current) {
          current = (current as Record<string, unknown>)[part]
        } else {
          // Fallback to vi dictionary if key is missing in active language
          let viCurrent: unknown = translations.vi
          for (const viPart of parts) {
            if (viCurrent && typeof viCurrent === 'object' && viPart in viCurrent) {
              viCurrent = (viCurrent as Record<string, unknown>)[viPart]
            } else {
              return fallback || path
            }
          }
          return typeof viCurrent === 'string' ? viCurrent : fallback || path
        }
      }

      return typeof current === 'string' ? current : fallback || path
    },
    [lang]
  )

  return <I18nContext.Provider value={{ lang, setLang, t }}>{children}</I18nContext.Provider>
}

export const useI18n = () => {
  const context = useContext(I18nContext)
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider')
  }
  return context
}
