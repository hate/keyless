import type { Config } from 'tailwindcss'

export default {
  content: [
    './index.html',
    './src/**/*.{ts,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        // Background colors
        bgCard: '#0f0f0f',
        bgInput: '#1a1a1a',
        bgRow: '#121212',
        bgPreview: '#141414',
        bgTrack: '#1c1c1c',
        bgHover: '#333333',
        bgHoverAlt: '#3a3a3a',
        // Border colors
        border: '#2a2a2a',
        borderLight: '#1c1c1c',
        borderPill: '#404040',
        borderPillHover: '#5a5a5a',
        borderError: '#4e1f1f',
        // Text colors
        textPrimary: '#e5e5e5',
        textAlt: '#e8e8e8',
        textSecondary: '#a0a0a0',
        textMuted: '#7a7a7a',
        textDisabled: '#6a6a6a',
        // Status colors
        statusSuccess: '#50fa7b',
        statusError: '#f87171',
        statusWarning: '#f1fa8c',
        statusInfo: '#4ea3ff',
        // Status backgrounds
        statusBgSuccess: '#0e2a14',
        statusBgError: '#2a1414',
        statusBgWarning: '#2a1a0e',
        statusBgInfo: '#0e1a2a',
        // Status text colors
        statusTextSuccess: '#a7e7b9',
        statusTextError: '#f4b5b5',
        statusTextWarning: '#f1fa8c',
        statusTextInfo: '#4ea3ff',
        // Error/warning variants
        error: '#ff5555',
        errorLight: '#ff6666',
        errorBg: '#2a1414',
        errorBorder: '#4e1f1f',
        errorText: '#ffbcbc',
        errorTextHover: '#ffdede',
        errorTextAlt: '#f59b9b',
        errorDanger: '#e93a3a',
        // Accent colors (for buttons, highlights)
        accentGreen: '#50fa7b',
        accentYellow: '#f1fa8c',
        accentBlue: '#4ea3ff',
        accentRed: '#ff5555',
        // Neutral colors
        white: '#ffffff',
        black: '#0f0f0f',
      },
      borderRadius: {
        xl: '12px',
      },
             keyframes: {
               blink: {
                 '0%, 50%': { opacity: '1' },
                 '51%, 100%': { opacity: '0' },
               },
               pulseBright: {
                 '0%, 100%': { opacity: '1', filter: 'brightness(1)' },
                 '50%': { opacity: '1', filter: 'brightness(1.5)' },
               },
               scaleIn: {
                 '0%': { transform: 'scale(0.9)', opacity: '0' },
                 '100%': { transform: 'scale(1)', opacity: '1' },
               },
             },
             animation: {
               blink: 'blink 1s steps(1, end) infinite',
               pulseBright: 'pulseBright 1.5s ease-in-out infinite',
               scaleIn: 'scaleIn 0.2s ease-out',
             },
    },
  },
  plugins: [],
} satisfies Config

