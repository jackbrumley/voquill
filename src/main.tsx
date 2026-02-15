import { render } from 'preact'
import './index.css'
import App from './App.tsx'
import Overlay from './Overlay.tsx'
import { tokens, tokensToCssVars } from './design-tokens.ts';

// Global Design Token Initialization
console.log('🎬 Voquill Frontend Initializing...');
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
  
  console.log('🚀 Voquill Routing Check:', { path, search, hash });

  // Handle various ways the path might be represented in a packaged app
  const isOverlay = 
    path.includes('overlay') || 
    search.includes('overlay') || 
    hash.includes('overlay');

  if (isOverlay) {
    console.log('🎭 Rendering: Overlay');
    return <Overlay />
  }

  console.log('🏠 Rendering: Main App');
  return <App />
}

console.log('⚛️ Attempting to render Preact root...');
const rootElement = document.getElementById('root');
if (!rootElement) {
  console.error('❌ CRITICAL: Root element #root not found in DOM!');
} else {
  render(<Main />, rootElement);
  console.log('✅ Preact render command issued');
}

