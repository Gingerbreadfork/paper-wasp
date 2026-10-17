import '@fontsource-variable/archivo/standard.css';
import '@fontsource-variable/martian-mono/standard.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { installDevConsole } from './lib/devlog';
import { installDevDrive } from './lib/devdrive';

if (import.meta.env.DEV) {
  installDevConsole();
  installDevDrive();
}

const app = mount(App, { target: document.getElementById('app')! });

export default app;
