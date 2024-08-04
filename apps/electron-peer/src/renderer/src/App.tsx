import Versions from './components/Versions'
import { Button } from './components/ui/button'

function App(): JSX.Element {
  const ipcHandle = (): void => window.electron.ipcRenderer.send('ping')

  return (
    <>
      <div className="text">
        Build an Electron app with <span className="react">React</span>
        &nbsp;and <span className="ts">TypeScript</span>
      </div>
      <p className="tip">
        Please try pressing <code>F12</code> to open the devTool
      </p>
      <Button variant="default" onClick={ipcHandle} formTarget="_blank" rel="noreferrer">
        Send IPC
      </Button>
      <Versions />
    </>
  )
}

export default App
