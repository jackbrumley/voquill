<script>
  import { onMount } from 'svelte';
  import { Greet, RecordAndTranscribe, StopRecording, GetHistory, GetConfig, SaveConfig, GetRecordingStatus } from '../wailsjs/go/main/App';

  let isRecording = false;
  let hotkeyPressed = false;
  let isTranscribing = false;
  let isTyping = false;
  let history = [];
  let config = {
    apiKey: '',
    hotkey: 'Ctrl+Shift+Alt',
    typingInterval: 0.05
  };
  let currentTab = 'status';
  let statusInterval;

  onMount(async () => {
    try {
      history = await GetHistory();
      config = await GetConfig();
      console.log("Greet test:", await Greet("Voquill"));
      
      // Start polling for recording status
      statusInterval = setInterval(async () => {
        try {
          const status = await GetRecordingStatus();
          isRecording = status.isRecording;
          hotkeyPressed = status.hotkeyPressed;
          isTranscribing = status.isTranscribing;
          isTyping = status.isTyping;
        } catch (error) {
          console.error("Error getting recording status:", error);
        }
      }, 100); // Poll every 100ms for responsive UI
    } catch (error) {
      console.error("Error loading initial data:", error);
    }
    
    return () => {
      if (statusInterval) {
        clearInterval(statusInterval);
      }
    };
  });

  async function handleRecord() {
    try {
      isRecording = true;
      const result = await RecordAndTranscribe();
      isRecording = false;
      if (result !== 'Already recording') {
        history = await GetHistory();
      }
    } catch (error) {
      console.error("Recording error:", error);
      isRecording = false;
    }
  }

  function handleStop() {
    try {
      StopRecording();
      isRecording = false;
    } catch (error) {
      console.error("Stop recording error:", error);
    }
  }

  async function handleSaveConfig() {
    try {
      await SaveConfig(config);
      console.log("Config saved successfully");
    } catch (error) {
      console.error("Save config error:", error);
    }
  }
</script>

<main class="app-main">
  <div class="app-container">
    <header class="app-header">
      <nav class="nav-container">
        <div class="nav-tabs">
          <button 
            class="nav-tab-compact {currentTab === 'status' ? 'active' : ''}" 
            on:click="{() => currentTab = 'status'}"
          >
            <svg class="nav-icon" fill="currentColor" viewBox="0 0 24 24">
              <path d="M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z"/>
            </svg>
          </button>
          <button 
            class="nav-tab-compact {currentTab === 'history' ? 'active' : ''}" 
            on:click="{() => currentTab = 'history'}"
          >
            <svg class="nav-icon" fill="currentColor" viewBox="0 0 24 24">
              <path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/>
            </svg>
          </button>
          <button 
            class="nav-tab-compact {currentTab === 'settings' ? 'active' : ''}" 
            on:click="{() => currentTab = 'settings'}"
          >
            <svg class="nav-icon" fill="currentColor" viewBox="0 0 24 24">
              <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"/>
            </svg>
          </button>
        </div>
      </nav>
    </header>

    <div class="content-area">
    {#if currentTab === 'status'}
      <section class="tab-section">
        <!-- Status Display -->
        <div class="tab-header">
          {#if isRecording}
            <h2>🔴 Recording...</h2>
          {:else if isTranscribing}
            <h2>🧠 Transcribing...</h2>
          {:else if isTyping}
            <h2>⌨️ Typing...</h2>
          {:else if hotkeyPressed}
            <h2>Ready to record</h2>
          {:else}
            <h2>✅ Ready</h2>
          {/if}
        </div>
        
        <!-- Compact Instructions -->
        <div class="instructions-card">
          <h3 class="instructions-title">Quick Guide</h3>
          <ol class="instructions-list">
            <li>Position cursor in target app</li>
            <li>Hold <kbd class="hotkey-display">{config.hotkey}</kbd></li>
            <li>Speak clearly</li>
            <li>Release hotkey to transcribe</li>
          </ol>
        </div>
      </section>
    {:else if currentTab === 'history'}
      <section class="tab-section">
        <div class="tab-header">
          <h2>History</h2>
        </div>
        <div class="tab-content">
          <div class="history-controls">
            <button class="btn-compact">
            <svg class="icon-small" fill="currentColor" viewBox="0 0 24 24">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
          </button>
        </div>
        
        {#if history.length === 0}
          <div class="empty-state">
            <div class="empty-icon">
              <svg class="icon-medium" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9z"/>
              </svg>
            </div>
            <h3 class="empty-title">No transcriptions yet</h3>
            <p class="empty-description">Start recording to see history</p>
            <button 
              class="btn-compact-primary"
              on:click="{() => currentTab = 'status'}"
            >
              Get Started
            </button>
          </div>
        {:else}
          <div class="history-list">
            {#each history as entry, index}
              <div class="history-item">
                <div class="history-header">
                  <div class="history-info">
                    <div class="history-number">
                      <span class="number-text">{index + 1}</span>
                    </div>
                    <div>
                      <p class="history-date">
                        {new Date(entry.Timestamp).toLocaleDateString()}
                      </p>
                    </div>
                  </div>
                  <div class="history-actions">
                    <button 
                      class="btn-mini"
                      on:click="{() => navigator.clipboard.writeText(entry.Text)}"
                      title="Copy"
                    >
                      <svg class="icon-tiny" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                      </svg>
                    </button>
                    <button 
                      class="btn-mini delete-btn"
                      title="Delete"
                    >
                      <svg class="icon-tiny" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                      </svg>
                    </button>
                  </div>
                </div>
                <div class="history-text">
                  <p class="text-content">{entry.Text}</p>
                </div>
                <div class="history-stats">
                  <span>{entry.Text.length} chars</span>
                  <span>{entry.Text.split(' ').length} words</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {:else if currentTab === 'settings'}
      <section class="tab-section">
        <div class="tab-header">
          <h2>Settings</h2>
        </div>
        <div class="tab-content">
          <div class="settings-grid">
            <!-- API Configuration -->
            <label for="apiKey">API Key</label>
            <input 
              type="password" 
              id="apiKey"
              class="input-field-compact" 
              placeholder="sk-..."
              bind:value="{config.apiKey}"
            >
            
            <!-- Hotkey Configuration -->
            <label for="hotkey">Hotkey</label>
            <input 
              type="text" 
              id="hotkey"
              class="input-field-compact" 
              placeholder="Ctrl+Shift+Alt"
              bind:value="{config.hotkey}"
            >
            
            <!-- Performance Settings -->
            <label for="typingInterval">Typing Speed</label>
            <input 
              type="number" 
              step="0.01" 
              min="0.01"
              max="1"
              id="typingInterval"
              class="input-field-compact" 
              bind:value="{config.typingInterval}"
            >
          </div>
          
          <!-- Save Button -->
          <div class="save-button-container">
            <button 
              class="btn-compact-primary save-button"
              on:click="{handleSaveConfig}"
            >
              <svg class="icon-tiny save-icon" fill="currentColor" viewBox="0 0 24 24">
                <path d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
              </svg>
              Save
            </button>
          </div>
        </div>
      </section>
    {/if}
  </div>
</main>
