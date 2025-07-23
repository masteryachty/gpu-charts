import { spawn, ChildProcess } from 'child_process';
import { createServer, Server } from 'http';
import { createServer as createHttpsServer } from 'https';
import path from 'path';

/**
 * Test Server Helper
 * 
 * Manages starting and stopping the test data server for Playwright tests
 */

export class TestServerHelper {
  private serverProcess: ChildProcess | null = null;
  private server: Server | null = null;
  private port: number;
  private isHttps: boolean;
  
  constructor(port = 8080, useHttps = false) {
    this.port = port;
    this.isHttps = useHttps;
  }
  
  /**
   * Start the test data server
   */
  async start(): Promise<void> {
    return new Promise((resolve, reject) => {
      const serverPath = path.join(__dirname, '../test-server.js');
      const args = [
        serverPath,
        '--port', this.port.toString(),
        '--host', 'localhost'
      ];
      
      if (!this.isHttps) {
        args.push('--http');
      }
      
      console.log(`[TestServerHelper] Starting test server on port ${this.port}`);
      
      this.serverProcess = spawn('node', args, {
        stdio: ['ignore', 'pipe', 'pipe'],
        detached: false
      });
      
      let started = false;
      
      this.serverProcess.stdout?.on('data', (data) => {
        const output = data.toString();
        console.log(`[TestServer] ${output.trim()}`);
        
        if (output.includes('Server running') && !started) {
          started = true;
          resolve();
        }
      });
      
      this.serverProcess.stderr?.on('data', (data) => {
        const error = data.toString();
        console.error(`[TestServer] Error: ${error.trim()}`);
        
        if (!started) {
          reject(new Error(`Test server failed to start: ${error}`));
        }
      });
      
      this.serverProcess.on('error', (error) => {
        console.error(`[TestServerHelper] Process error:`, error);
        if (!started) {
          reject(error);
        }
      });
      
      this.serverProcess.on('exit', (code, signal) => {
        console.log(`[TestServerHelper] Server process exited with code ${code}, signal ${signal}`);
        this.serverProcess = null;
      });
      
      // Timeout after 10 seconds
      setTimeout(() => {
        if (!started) {
          reject(new Error('Test server startup timeout'));
        }
      }, 10000);
    });
  }
  
  /**
   * Stop the test data server
   */
  async stop(): Promise<void> {
    return new Promise((resolve) => {
      if (this.serverProcess) {
        console.log('[TestServerHelper] Stopping test server');
        
        this.serverProcess.on('exit', () => {
          this.serverProcess = null;
          resolve();
        });
        
        // Try graceful shutdown first
        this.serverProcess.kill('SIGTERM');
        
        // Force kill after 5 seconds if still running
        setTimeout(() => {
          if (this.serverProcess) {
            console.log('[TestServerHelper] Force killing test server');
            this.serverProcess.kill('SIGKILL');
            this.serverProcess = null;
          }
          resolve();
        }, 5000);
      } else {
        resolve();
      }
    });
  }
  
  /**
   * Check if the server is running
   */
  async isRunning(): Promise<boolean> {
    try {
      const protocol = this.isHttps ? 'https' : 'http';
      const response = await fetch(`${protocol}://localhost:${this.port}/health`);
      return response.ok;
    } catch {
      return false;
    }
  }
  
  /**
   * Get the server URL
   */
  getUrl(): string {
    const protocol = this.isHttps ? 'https' : 'http';
    return `${protocol}://localhost:${this.port}`;
  }
  
  /**
   * Wait for server to be ready
   */
  async waitForReady(timeout = 10000): Promise<void> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      if (await this.isRunning()) {
        console.log('[TestServerHelper] Server is ready');
        return;
      }
      
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    throw new Error(`Test server not ready after ${timeout}ms`);
  }
}

/**
 * Global test server instance for Playwright configuration
 */
let globalTestServer: TestServerHelper | null = null;

export async function startGlobalTestServer(port = 8080, useHttps = false): Promise<TestServerHelper> {
  if (globalTestServer) {
    return globalTestServer;
  }
  
  globalTestServer = new TestServerHelper(port, useHttps);
  await globalTestServer.start();
  await globalTestServer.waitForReady();
  
  return globalTestServer;
}

export async function stopGlobalTestServer(): Promise<void> {
  if (globalTestServer) {
    await globalTestServer.stop();
    globalTestServer = null;
  }
}

export function getGlobalTestServer(): TestServerHelper | null {
  return globalTestServer;
}