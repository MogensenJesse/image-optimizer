import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { Command } from '@tauri-apps/plugin-shell';
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [sidecarResponse, setSidecarResponse] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  async function executeSidecar() {
    try {
      console.log('=== Starting Sidecar Execution ===');
      
      // Add more detailed logging
      const command = Command.sidecar('binaries/test', ['ping', name]);
      console.log('Command object:', command);
      
      console.log('Executing command...');
      const output = await command.execute();
      console.log('Command output:', output);
      
      setSidecarResponse(output.stdout);
      
    } catch (error) {
      console.error('=== Sidecar Execution Failed ===');
      console.error('Error details:', {
        name: error.name,
        message: error.message,
        code: error.code,
        status: error.status,
        signal: error.signal,
        stdout: error.stdout,
        stderr: error.stderr
      });
      setSidecarResponse(`Error: ${error.message}`);
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          console.log('\n=== Form Submitted ===');
          console.log('Input name:', name);
          greet();
          executeSidecar();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>

      <p>{greetMsg}</p>
      <p>Sidecar Response: {sidecarResponse}</p>
    </main>
  );
}

export default App;
