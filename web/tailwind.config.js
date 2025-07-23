/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // High contrast black theme
        bg: {
          primary: '#000000',
          secondary: '#0a0a0a',
          tertiary: '#141414',
          quaternary: '#1f1f1f',
        },
        accent: {
          green: '#00ff41',
          red: '#ff073a',
          blue: '#0080ff',
          yellow: '#ffff00',
          purple: '#8b5cf6',
        },
        text: {
          primary: '#ffffff',
          secondary: '#cccccc',
          tertiary: '#888888',
          disabled: '#444444',
        },
        border: '#333333',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
        display: ['Cal Sans', 'Inter', 'sans-serif'],
      },
      animation: {
        'glow': 'glow 2s ease-in-out infinite alternate',
        'pulse-green': 'pulse-green 0.5s ease-in-out',
        'pulse-red': 'pulse-red 0.5s ease-in-out',
      },
      keyframes: {
        glow: {
          '0%': { boxShadow: '0 0 5px theme(colors.accent.blue)' },
          '100%': { boxShadow: '0 0 20px theme(colors.accent.blue)' },
        },
        'pulse-green': {
          '0%, 100%': { backgroundColor: 'transparent' },
          '50%': { backgroundColor: 'theme(colors.accent.green / 0.2)' },
        },
        'pulse-red': {
          '0%, 100%': { backgroundColor: 'transparent' },
          '50%': { backgroundColor: 'theme(colors.accent.red / 0.2)' },
        },
      },
    },
  },
  plugins: [],
}