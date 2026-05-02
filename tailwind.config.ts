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
          background: '#10131a',
          panel: '#171b24',
          panelSoft: '#202636',
          border: '#2f3848',
          text: '#e8edf5',
          muted: '#9aa7b8',
          accent: '#9bb7ff',
          accentSoft: '#263657',
          success: '#7ddfb3',
          warning: '#ffd166',
          error: '#ff9aa2',
        },
      },
    },
  },
  plugins: [],
} satisfies Config;
