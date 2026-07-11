import { createApp } from 'vue'
import App from './App.vue'

// Import Vuetify
import 'vuetify/styles'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import '@mdi/font/css/materialdesignicons.css' // Import MDI icon styles
import './styles/theme.scss' // Global theme refinements (Graphite & Cinnabar)

// Import i18n
import i18n from './i18n'
import { usePanelManager } from './composables/usePanelManager'

const vuetify = createVuetify({
  components,
  directives,
  icons: {
    defaultSet: 'mdi', // Set default icon set to mdi
  },
  // Theme: "Graphite & Cinnabar" — a professional analysis-workbench palette.
  // The single accent is cinnabar red (朱砂), the traditional ink of the red 帥,
  // culturally true to Jieqi. Everything else is disciplined neutral graphite so
  // the board and the capture pool read as the focus.
  theme: {
    defaultTheme: 'light',
    themes: {
      light: {
        dark: false,
        colors: {
          primary: '#C0362C', // cinnabar
          secondary: '#6B6862', // soft ink
          accent: '#C0362C',
          error: '#B3261E',
          info: '#3C6E9C',
          success: '#3F9B5B',
          warning: '#C98A2B',
          background: '#F1F0EC', // warm paper-gray
          surface: '#FFFFFF',
          button: '#FFFFFF',
          'on-background': '#1C1B19',
          'on-surface': '#1C1B19',
          'on-primary': '#FFFFFF',
        },
      },
      dark: {
        dark: true,
        colors: {
          primary: '#E4483B', // cinnabar, brighter for dark
          secondary: '#8B9099',
          accent: '#E4483B',
          error: '#E4483B',
          info: '#5C9BD1',
          success: '#4CAF6E',
          warning: '#D9A441',
          background: '#16181D', // graphite
          surface: '#1E2127',
          button: '#262A31',
          'on-background': '#E8E6E1',
          'on-surface': '#E8E6E1',
          'on-primary': '#FFFFFF',
        },
      },
    },
  },
})

const app = createApp(App)

// Initialize i18n
app.use(i18n)

// Initialize Vuetify
app.use(vuetify)

// Initialize panel manager
const panelManager = usePanelManager()
panelManager.initialize()

// Mount the app
app.mount('#app')
