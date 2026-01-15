import { render } from 'preact'
import './index.css'
import App from './App.tsx'
import Overlay from './Overlay.tsx'

const Main = () => {
  const path = window.location.pathname
  console.log('🚀 Current path:', path);
  if (path === '/overlay' || path.includes('overlay')) {
    return <Overlay />
  }
  return <App />
}

render(<Main />, document.getElementById('root')!)
