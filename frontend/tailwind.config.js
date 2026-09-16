/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      spacing: {
        '4.5': '1.125rem',
        '6.5': '1.625rem',
        '10.5': '2.625rem',
      },
      colors: {
        bg: {
          base: 'hsl(var(--color-bg-base) / <alpha-value>)',
          elevated: 'hsl(var(--color-bg-elevated) / <alpha-value>)',
          card: 'hsl(var(--color-bg-card) / <alpha-value>)',
          'card-hover': 'hsl(var(--color-bg-card-hover) / <alpha-value>)',
          input: 'hsl(var(--color-bg-input) / <alpha-value>)',
        },
        border: {
          DEFAULT: 'hsl(var(--color-border) / calc(<alpha-value> * 0.08))',
          strong: 'hsl(var(--color-border-strong) / calc(<alpha-value> * 0.18))',
        },
        text: {
          primary: 'hsl(var(--color-text-primary) / <alpha-value>)',
          secondary: 'hsl(var(--color-text-secondary) / <alpha-value>)',
          muted: 'hsl(var(--color-text-muted) / <alpha-value>)',
          inverse: 'hsl(var(--color-text-inverse) / <alpha-value>)',
        },
        accent: {
          DEFAULT: 'hsl(var(--color-accent) / <alpha-value>)',
          dim: 'hsl(var(--color-accent) / calc(<alpha-value> * 0.12))',
          hover: 'hsl(var(--color-accent-hover) / <alpha-value>)',
        },
        success: {
          DEFAULT: 'hsl(var(--color-success) / <alpha-value>)',
          bg: 'hsl(var(--color-success) / calc(<alpha-value> * 0.13))',
        },
        warning: {
          DEFAULT: 'hsl(var(--color-warning) / <alpha-value>)',
          bg: 'hsl(var(--color-warning) / calc(<alpha-value> * 0.13))',
        },
        error: {
          DEFAULT: 'hsl(var(--color-error) / <alpha-value>)',
          bg: 'hsl(var(--color-error) / calc(<alpha-value> * 0.12))',
        },
        info: {
          DEFAULT: 'hsl(var(--color-info) / <alpha-value>)',
          bg: 'hsl(var(--color-info) / calc(<alpha-value> * 0.12))',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Geist Mono', 'ui-monospace', 'monospace'],
      },
      borderRadius: {
        sm: 'var(--radius-sm)',
        md: 'var(--radius-md)',
        lg: 'var(--radius-lg)',
        xl: 'var(--radius-xl)',
      },
      boxShadow: {
        xs: '0 1px 2px rgba(0, 0, 0, 0.05)',
        card: 'var(--shadow-sm)',
        float: 'var(--shadow-md)',
        modal: 'var(--shadow-lg)',
      },
      backdropBlur: {
        xs: '2px',
      },
      transitionDuration: {
        120: '120ms',
      },
      transitionTimingFunction: {
        'out-expo': 'cubic-bezier(0.16, 1, 0.3, 1)',
        smooth: 'cubic-bezier(0.4, 0, 0.2, 1)',
      },
    },
  },
  plugins: [require('tailwindcss-animate')],
}
