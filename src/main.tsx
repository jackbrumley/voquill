import { render } from 'preact'
import './index.css'
import App from './App.tsx'
import Overlay from './Overlay.tsx'

const Main = () => {
  const path = window.location.pathname
  if (path === '/overlay') {
    return <Overlay />
  }
  return <App />
}

render(<Main />, document.getElementById('root')!)
