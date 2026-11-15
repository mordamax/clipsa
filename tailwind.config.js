import forms from '@tailwindcss/forms';

/** Semantic colours backed by the CSS variables in src/app.css (light + dark). */
const token = (name) => `rgb(var(--c-${name}) / <alpha-value>)`;

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Indigo, matching clipsa.dev.
        primary: {
          50: '#eef2ff',
          100: '#e0e7ff',
          200: '#c7d2fe',
          300: '#a5b4fc',
          400: '#818cf8',
          500: '#6366f1',
          600: '#4f46e5',
          700: '#4338ca',
          800: '#3730a3',
          900: '#312e81',
          950: '#1e1b4b'
        },
        canvas: token('canvas'),
        sidebar: token('sidebar'),
        surface: token('surface'),
        elevated: token('elevated'),
        field: token('field'),
        hover: token('hover'),
        line: token('line'),
        'line-strong': token('line-strong'),
        fg: token('fg'),
        'fg-2': token('fg-2'),
        muted: token('muted'),
        subtle: token('subtle'),
        accent: token('accent'),
        'accent-text': token('accent-text'),
        cyan: token('cyan'),
        success: token('success'),
        danger: token('danger'),
        warning: token('warning')
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          '"SF Pro Text"',
          '"Segoe UI Variable Text"',
          '"Segoe UI"',
          'system-ui',
          'Roboto',
          '"Helvetica Neue"',
          'Arial',
          'sans-serif'
        ],
        mono: ['ui-monospace', '"SF Mono"', 'SFMono-Regular', 'Menlo', 'Consolas', '"Liberation Mono"', 'monospace']
      },
      // Desktop-app type scale (13px body, like native macOS/Windows apps).
      fontSize: {
        '2xs': ['10.5px', '14px'],
        xs: ['11.5px', '16px'],
        sm: ['13px', '18px'],
        base: ['14px', '20px'],
        lg: ['16px', '22px'],
        xl: ['19px', '26px'],
        '2xl': ['24px', '30px']
      },
      boxShadow: {
        pop: '0 12px 32px -12px rgb(0 0 0 / 0.35), 0 2px 6px -2px rgb(0 0 0 / 0.12)',
        glow: '0 10px 30px -12px rgb(90 84 238 / 0.85)'
      }
    }
  },
  plugins: [forms]
};
