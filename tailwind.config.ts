import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Geist Sans', 'system-ui', 'sans-serif'],
        mono: ['Geist Mono', 'JetBrains Mono', 'monospace'],
      },
      colors: {
        app: {
          background: 'rgb(var(--color-app-background) / <alpha-value>)',
          panel: 'rgb(var(--color-app-panel) / <alpha-value>)',
          panelSoft: 'rgb(var(--color-app-panel-soft) / <alpha-value>)',
          border: 'rgb(var(--color-app-border) / <alpha-value>)',
          text: 'rgb(var(--color-app-text) / <alpha-value>)',
          muted: 'rgb(var(--color-app-muted) / <alpha-value>)',
          accent: 'rgb(var(--color-app-accent) / <alpha-value>)',
          accentSoft: 'rgb(var(--color-app-accent-soft) / <alpha-value>)',
          success: 'rgb(var(--color-app-success) / <alpha-value>)',
          warning: 'rgb(var(--color-app-warning) / <alpha-value>)',
          error: 'rgb(var(--color-app-error) / <alpha-value>)',
        },
      },
    },
  },
  plugins: [],
} satisfies Config;
