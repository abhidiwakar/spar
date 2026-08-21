/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: {
          950: "#0c0c0e",
          900: "#141417",
          850: "#1a1a1f",
          800: "#222228",
          700: "#2c2c34",
        },
        paper: {
          100: "#e8e6e3",
          400: "#9a9690",
          500: "#6f6b66",
        },
        gold: {
          400: "#e8a54b",
          500: "#d4892a",
          700: "#8a5a18",
        },
        easy: "#3d9a5f",
        medium: "#c9a227",
        hard: "#d4534f",
        run: "#3d6ea8",
      },
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "Segoe UI",
          "sans-serif",
        ],
        serif: ["Iowan Old Style", "Palatino Linotype", "Palatino", "Georgia", "serif"],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas", "monospace"],
      },
    },
  },
  plugins: [],
};
