import type { Options } from '@wdio/types'
import { spawn, spawnSync } from 'child_process'

// Tauriアプリのパスを環境に応じて設定
const tauriDriver = process.platform === 'win32' 
  ? 'tauri-driver.exe' 
  : 'tauri-driver'

export const config: Options.Testrunner = {
  specs: ['./tests/e2e/specs/**/*.e2e.ts'],
  exclude: [],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: '../../src-tauri/target/release/kukuri-tauri',
      },
    } as any,
  ],
  logLevel: 'info',
  bail: 0,
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  
  beforeSession: () => {
    // tauri-driverが利用可能か確認
    const checkDriver = spawnSync(tauriDriver, ['--version'])
    if (checkDriver.error) {
      console.error('tauri-driver is not installed. Please run: cargo install tauri-driver')
      process.exit(1)
    }
  },
  
  onPrepare: () => {
    // Tauriドライバーを起動
    const driverProcess = spawn(
      tauriDriver,
      [],
      { stdio: [null, process.stdout, process.stderr] }
    )
    
    return new Promise<void>((resolve) => {
      driverProcess.stdout?.on('data', (data) => {
        if (data.toString().includes('Listening on')) {
          resolve()
        }
      })
    })
  },
}