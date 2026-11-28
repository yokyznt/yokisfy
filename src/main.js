import '@fontsource/geist-sans/400.css'
import '@fontsource/geist-sans/500.css'
import '@fontsource/geist-sans/600.css'
import '@fontsource/geist-sans/700.css'
import '@fontsource/geist-mono/400.css'
import '@fontsource/geist-mono/500.css'
import './app.css'
import { mount } from 'svelte'
import App from './App.svelte'
import { applyLook } from './lib/state.svelte.js'

applyLook()

mount(App, { target: document.getElementById('app') })
