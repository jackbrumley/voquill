import { render } from 'preact'
import './index.css'
import App from './App.tsx'
import Overlay from './Overlay.tsx'
import { tokens, tokensToCssVars } from './design-tokens.ts';

// Global Design Token Initialization
// Injects CSS variables into the root element so they are available to all windows (App, Overlay, etc.)
const initDesignTokens = () => {
  const cssVars = tokensToCssVars(tokens);
  const root = document.documentElement;
  Object.entries(cssVars).forEach(([key, value]) => {
    root.style.setProperty(key, value as string);
  });
};

initDesignTokens();

const Main = () => {

  const path = window.location.pathname
  console.log('🚀 Current path:', path);
  if (path === '/overlay' || path.includes('overlay')) {
    return <Overlay />
  }
  return <App />
}

render(<Main />, document.getElementById('root')!)
