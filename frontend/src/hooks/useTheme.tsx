import React, { createContext, useContext, useEffect, useState, useCallback } from "react"

export type ThemeMode = "system" | "dark" | "light"

interface ThemeContextType {
  theme: ThemeMode
  resolvedTheme: "dark" | "light"
  setTheme: (theme: ThemeMode) => void
  toggleTheme: () => void
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined)

const STORAGE_KEY = "502drive-theme"

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [theme, setThemeState] = useState<ThemeMode>(() => {
    const saved = localStorage.getItem(STORAGE_KEY) as ThemeMode | null
    if (saved === "dark" || saved === "light" || saved === "system") {
      return saved
    }
    return "system"
  })

  const [resolvedTheme, setResolvedTheme] = useState<"dark" | "light">("dark")

  const applyTheme = useCallback((currentTheme: ThemeMode) => {
    const root = document.documentElement
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches
    const effective = currentTheme === "system" ? (prefersDark ? "dark" : "light") : currentTheme

    // Add transitioning class for 300ms smooth Apple-style easing
    root.classList.add("theme-transitioning")

    const updateClasses = () => {
      setResolvedTheme(effective)
      if (effective === "dark") {
        root.classList.add("dark")
        root.classList.remove("light")
      } else {
        root.classList.add("light")
        root.classList.remove("dark")
      }
    }

    updateClasses()

    const timer = setTimeout(() => {
      root.classList.remove("theme-transitioning")
    }, 320)

    return () => clearTimeout(timer)
  }, [])

  useEffect(() => {
    applyTheme(theme)

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)")
    const handleChange = () => {
      if (theme === "system") {
        applyTheme("system")
      }
    }

    mediaQuery.addEventListener("change", handleChange)
    return () => mediaQuery.removeEventListener("change", handleChange)
  }, [theme, applyTheme])

  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme)
    localStorage.setItem(STORAGE_KEY, newTheme)
    applyTheme(newTheme)
  }

  const toggleTheme = () => {
    const next = resolvedTheme === "dark" ? "light" : "dark"
    setTheme(next)
  }

  return (
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export const useTheme = () => {
  const context = useContext(ThemeContext)
  if (!context) {
    throw new Error("useTheme must be used within a ThemeProvider")
  }
  return context
}
