import type { Config } from 'tailwindcss'

export default {
  content: [
    './index.html',
    './src/**/*.{ts,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        // Map to our TUI palette
        card: '#0f0f0f',
        border: '#2a2a2a',
        textPrimary: '#e5e5e5',
        textSecondary: '#a0a0a0',
      },
      borderRadius: {
        xl: '12px',
      },
      keyframes: {
        blink: {
          '0%, 50%': { opacity: '1' },
          '51%, 100%': { opacity: '0' },
        },
      },
      animation: {
        blink: 'blink 1s steps(1, end) infinite',
      },
    },
  },
  plugins: [],
} satisfies Config


