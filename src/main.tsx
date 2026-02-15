import { render } from 'preact'
import './index.css'
import App from './App.tsx'
import Overlay from './Overlay.tsx'
import { tokens, tokensToCssVars } from './design-tokens.ts';

// Global Design Token Initialization
const initDesignTokens = () => {
  const cssVars = tokensToCssVars(tokens);
  const root = document.documentElement;
  Object.entries(cssVars).forEach(([key, value]) => {
    root.style.setProperty(key, value as string);
  });
};

initDesignTokens();

const Main = () => {
  const path = window.location.pathname;
  const search = window.location.search;
  const hash = window.location.hash;

  // Handle various ways the path might be represented in a packaged app
  const isOverlay = 
    path.includes('overlay') || 
    search.includes('overlay') || 
    hash.includes('overlay');

  if (isOverlay) {
    return <Overlay />
  }

  return <App />
}

const rootElement = document.getElementById('root');
if (rootElement) {
  render(<Main />, rootElement);
}


