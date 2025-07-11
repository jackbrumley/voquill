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

<main class="min-h-screen">
  <div class="container mx-auto p-8 max-w-5xl">
    <header class="mb-12">
      <div class="text-center mb-8">
        <div class="inline-flex items-center justify-center w-20 h-20 bg-gradient-to-br from-primary-500 to-accent-500 rounded-2xl shadow-large mb-6">
          <svg class="w-10 h-10 text-white" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2C13.1 2 14 2.9 14 4C14 5.1 13.1 6 12 6C10.9 6 10 5.1 10 4C10 2.9 10.9 2 12 2ZM21 9V7L15 1H5C3.89 1 3 1.89 3 3V21C3 22.11 3.89 23 5 23H11V21H5V3H13V9H21ZM14 15.5L22.5 7L21 5.5L14 12.5L10.5 9L9 10.5L14 15.5Z"/>
          </svg>
        </div>
        <h1 class="text-5xl font-bold bg-gradient-to-r from-slate-800 to-slate-600 bg-clip-text text-transparent mb-3">
          Voquill
        </h1>
        <p class="text-lg text-slate-600 font-medium">Voice-to-Text Transcription</p>
      </div>
      
      <nav class="glass-card rounded-2xl p-2 max-w-md mx-auto">
        <div class="flex space-x-1">
          <button 
            class="nav-tab {currentTab === 'status' ? 'active' : ''}" 
            on:click="{() => currentTab = 'status'}"
          >
            <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
              <path d="M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z"/>
            </svg>
            Status
          </button>
          <button 
            class="nav-tab {currentTab === 'history' ? 'active' : ''}" 
            on:click="{() => currentTab = 'history'}"
          >
            <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
              <path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/>
            </svg>
            History
          </button>
          <button 
            class="nav-tab {currentTab === 'settings' ? 'active' : ''}" 
            on:click="{() => currentTab = 'settings'}"
          >
            <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
              <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"/>
            </svg>
            Settings
          </button>
        </div>
      </nav>
    </header>

    {#if currentTab === 'status'}
      <section class="glass-card rounded-3xl p-10 shadow-large">
        <div class="text-center mb-10">
          <!-- Status Display -->
          <div class="mb-8">
            {#if isRecording}
              <div class="status-indicator bg-gradient-to-r from-red-500 to-pink-500 text-white shadow-large mb-6">
                <div class="w-3 h-3 bg-white rounded-full mr-3 animate-pulse-soft"></div>
                <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2c1.1 0 2 .9 2 2v6c0 1.1-.9 2-2 2s-2-.9-2-2V4c0-1.1.9-2 2-2zm5.3 4.7c-.4-.4-1-.4-1.4 0s-.4 1 0 1.4C17.2 9.4 18 10.6 18 12s-.8 2.6-2.1 3.9c-.4.4-.4 1 0 1.4.2.2.5.3.7.3s.5-.1.7-.3C19.1 15.5 20 13.8 20 12s-.9-3.5-2.7-5.3zM4 9v6h4l5 5V4L8 9H4z"/>
                </svg>
                Recording in progress...
              </div>
            {:else if isTranscribing}
              <div class="status-indicator bg-gradient-to-r from-blue-500 to-indigo-500 text-white shadow-large mb-6">
                <div class="w-3 h-3 bg-white rounded-full mr-3 animate-pulse-soft"></div>
                <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                Transcribing audio...
              </div>
            {:else if isTyping}
              <div class="status-indicator bg-gradient-to-r from-purple-500 to-violet-500 text-white shadow-large mb-6">
                <div class="w-3 h-3 bg-white rounded-full mr-3 animate-pulse-soft"></div>
                <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M20 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm-5 14H9v-2h6v2zm3-4H6v-2h12v2zm0-4H6V8h12v2z"/>
                </svg>
                Typing text...
              </div>
            {:else if hotkeyPressed}
              <div class="status-indicator bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-large mb-6">
                <div class="w-3 h-3 bg-white rounded-full mr-3 animate-pulse-soft"></div>
                <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
                </svg>
                Hotkey detected - ready to record
              </div>
            {:else}
              <div class="status-indicator bg-gradient-to-r from-emerald-500 to-teal-500 text-white shadow-large mb-6">
                <div class="w-3 h-3 bg-white rounded-full mr-3 animate-pulse-soft"></div>
                <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                Ready for dictation
              </div>
            {/if}
          </div>
          
          <!-- Large Status Icon -->
          <div class="mb-8">
            <div class="inline-flex items-center justify-center w-32 h-32 rounded-full bg-gradient-to-br from-slate-100 to-slate-200 shadow-large mb-4">
              {#if isRecording}
                <svg class="w-16 h-16 text-red-500 animate-pulse" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2c1.1 0 2 .9 2 2v6c0 1.1-.9 2-2 2s-2-.9-2-2V4c0-1.1.9-2 2-2zm5.3 4.7c-.4-.4-1-.4-1.4 0s-.4 1 0 1.4C17.2 9.4 18 10.6 18 12s-.8 2.6-2.1 3.9c-.4.4-.4 1 0 1.4.2.2.5.3.7.3s.5-.1.7-.3C19.1 15.5 20 13.8 20 12s-.9-3.5-2.7-5.3z"/>
                </svg>
              {:else if isTranscribing}
                <svg class="w-16 h-16 text-blue-500 animate-spin" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                </svg>
              {:else if isTyping}
                <svg class="w-16 h-16 text-purple-500" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M20 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm-5 14H9v-2h6v2zm3-4H6v-2h12v2zm0-4H6V8h12v2z"/>
                </svg>
              {:else}
                <svg class="w-16 h-16 text-primary-500" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2c1.1 0 2 .9 2 2v6c0 1.1-.9 2-2 2s-2-.9-2-2V4c0-1.1.9-2 2-2z"/>
                </svg>
              {/if}
            </div>
          </div>
        </div>
        
        <!-- Instructions Card -->
        <div class="glass-card rounded-2xl p-8 max-w-2xl mx-auto">
          <div class="text-center mb-6">
            <div class="inline-flex items-center justify-center w-12 h-12 bg-gradient-to-br from-primary-500 to-accent-500 rounded-xl mb-4">
              <svg class="w-6 h-6 text-white" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 2c1.1 0 2 .9 2 2v6c0 1.1-.9 2-2 2s-2-.9-2-2V4c0-1.1.9-2 2-2z"/>
              </svg>
            </div>
            <h3 class="text-2xl font-bold text-slate-800 mb-2">How to Use Voquill</h3>
            <p class="text-slate-600">Follow these simple steps to start voice transcription</p>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="flex items-start space-x-4">
              <div class="flex-shrink-0 w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-600 rounded-full flex items-center justify-center text-white font-bold text-sm">1</div>
              <div>
                <h4 class="font-semibold text-slate-800 mb-1">Position Cursor</h4>
                <p class="text-slate-600 text-sm">Place your cursor where you want the transcribed text to appear</p>
              </div>
            </div>
            
            <div class="flex items-start space-x-4">
              <div class="flex-shrink-0 w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-600 rounded-full flex items-center justify-center text-white font-bold text-sm">2</div>
              <div>
                <h4 class="font-semibold text-slate-800 mb-1">Press Hotkey</h4>
                <p class="text-slate-600 text-sm">Hold <kbd class="bg-slate-200 px-2 py-1 rounded text-xs font-mono">{config.hotkey}</kbd> to start recording</p>
              </div>
            </div>
            
            <div class="flex items-start space-x-4">
              <div class="flex-shrink-0 w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-600 rounded-full flex items-center justify-center text-white font-bold text-sm">3</div>
              <div>
                <h4 class="font-semibold text-slate-800 mb-1">Speak Clearly</h4>
                <p class="text-slate-600 text-sm">Speak your message clearly and at a normal pace</p>
              </div>
            </div>
            
            <div class="flex items-start space-x-4">
              <div class="flex-shrink-0 w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-600 rounded-full flex items-center justify-center text-white font-bold text-sm">4</div>
              <div>
                <h4 class="font-semibold text-slate-800 mb-1">Release & Transcribe</h4>
                <p class="text-slate-600 text-sm">Release the hotkey to stop recording and start transcription</p>
              </div>
            </div>
          </div>
        </div>
      </section>
    {:else if currentTab === 'history'}
      <section class="glass-card rounded-3xl p-10 shadow-large">
        <div class="flex justify-between items-center mb-8">
          <div>
            <h2 class="text-3xl font-bold text-slate-800 mb-2">Transcription History</h2>
            <p class="text-slate-600">View and manage your voice transcription history</p>
          </div>
          <button class="btn-secondary group">
            <svg class="w-4 h-4 mr-2 group-hover:text-red-500 transition-colors" fill="currentColor" viewBox="0 0 24 24">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
            Clear All
          </button>
        </div>
        
        {#if history.length === 0}
          <div class="text-center py-16">
            <div class="inline-flex items-center justify-center w-20 h-20 bg-gradient-to-br from-slate-100 to-slate-200 rounded-2xl mb-6">
              <svg class="w-10 h-10 text-slate-400" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9z"/>
              </svg>
            </div>
            <h3 class="text-xl font-semibold text-slate-800 mb-2">No transcriptions yet</h3>
            <p class="text-slate-600 mb-6">Start using voice transcription to see your history here</p>
            <button 
              class="btn-primary"
              on:click="{() => currentTab = 'status'}"
            >
              <svg class="w-5 h-5 mr-2" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 2c1.1 0 2 .9 2 2v6c0 1.1-.9 2-2 2s-2-.9-2-2V4c0-1.1.9-2 2-2z"/>
              </svg>
              Get Started
            </button>
          </div>
        {:else}
          <div class="space-y-4 max-h-96 overflow-y-auto">
            {#each history as entry, index}
              <div class="glass-card rounded-2xl p-6 hover:shadow-medium transition-all duration-300 group">
                <div class="flex justify-between items-start mb-4">
                  <div class="flex items-center space-x-3">
                    <div class="w-10 h-10 bg-gradient-to-br from-primary-500 to-accent-500 rounded-xl flex items-center justify-center">
                      <span class="text-white font-bold text-sm">{index + 1}</span>
                    </div>
                    <div>
                      <p class="text-slate-500 text-sm font-medium">
                        {new Date(entry.Timestamp).toLocaleDateString()}
                      </p>
                      <p class="text-slate-400 text-xs">
                        {new Date(entry.Timestamp).toLocaleTimeString()}
                      </p>
                    </div>
                  </div>
                  <div class="flex space-x-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                    <button 
                      class="btn-secondary !py-2 !px-3 !text-sm"
                      on:click="{() => navigator.clipboard.writeText(entry.Text)}"
                      title="Copy to clipboard"
                    >
                      <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                      </svg>
                    </button>
                    <button 
                      class="btn-secondary !py-2 !px-3 !text-sm hover:!text-red-600 hover:!border-red-200"
                      title="Delete entry"
                    >
                      <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                      </svg>
                    </button>
                  </div>
                </div>
                <div class="bg-slate-50 rounded-xl p-4 border border-slate-100">
                  <p class="text-slate-800 leading-relaxed">{entry.Text}</p>
                </div>
                <div class="mt-4 flex items-center justify-between text-xs text-slate-400">
                  <span>{entry.Text.length} characters</span>
                  <span>{entry.Text.split(' ').length} words</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    {:else if currentTab === 'settings'}
      <section class="glass-card rounded-3xl p-10 shadow-large">
        <div class="mb-8">
          <h2 class="text-3xl font-bold text-slate-800 mb-2">Settings</h2>
          <p class="text-slate-600">Configure your Voquill preferences and API settings</p>
        </div>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
          <!-- API Configuration -->
          <div class="glass-card rounded-2xl p-6">
            <div class="flex items-center mb-6">
              <div class="w-10 h-10 bg-gradient-to-br from-blue-500 to-indigo-500 rounded-xl flex items-center justify-center mr-3">
                <svg class="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4z"/>
                </svg>
              </div>
              <div>
                <h3 class="text-xl font-bold text-slate-800">API Configuration</h3>
                <p class="text-slate-600 text-sm">OpenAI API settings for transcription</p>
              </div>
            </div>
            
            <div class="space-y-4">
              <div>
                <label for="apiKey" class="block text-slate-700 font-medium mb-2">
                  API Key
                </label>
                <div class="relative">
                  <input 
                    type="password" 
                    id="apiKey" 
                    class="input-field pr-12" 
                    placeholder="sk-..."
                    bind:value="{config.apiKey}"
                  >
                  <div class="absolute inset-y-0 right-0 flex items-center pr-3">
                    <svg class="w-5 h-5 text-slate-400" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4z"/>
                    </svg>
                  </div>
                </div>
                <p class="text-xs text-slate-500 mt-1">Your API key is stored securely and never shared</p>
              </div>
            </div>
          </div>
          
          <!-- Hotkey Configuration -->
          <div class="glass-card rounded-2xl p-6">
            <div class="flex items-center mb-6">
              <div class="w-10 h-10 bg-gradient-to-br from-purple-500 to-violet-500 rounded-xl flex items-center justify-center mr-3">
                <svg class="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M20 5H4c-1.1 0-1.99.9-1.99 2L2 17c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm-5 3h5v4h-5V8zm-1 4H9V8h5v4zm-6 0H4V8h4v4z"/>
                </svg>
              </div>
              <div>
                <h3 class="text-xl font-bold text-slate-800">Hotkey Settings</h3>
                <p class="text-slate-600 text-sm">Global keyboard shortcuts</p>
              </div>
            </div>
            
            <div class="space-y-4">
              <div>
                <label for="hotkey" class="block text-slate-700 font-medium mb-2">
                  Recording Hotkey
                </label>
                <div class="relative">
                  <input 
                    type="text" 
                    id="hotkey" 
                    class="input-field pr-12" 
                    placeholder="Ctrl+Shift+Alt"
                    bind:value="{config.hotkey}"
                  >
                  <div class="absolute inset-y-0 right-0 flex items-center pr-3">
                    <kbd class="bg-slate-200 px-2 py-1 rounded text-xs font-mono text-slate-600">
                      {config.hotkey || 'Not set'}
                    </kbd>
                  </div>
                </div>
                <p class="text-xs text-slate-500 mt-1">Hold this key combination to start recording</p>
              </div>
            </div>
          </div>
          
          <!-- Performance Settings -->
          <div class="glass-card rounded-2xl p-6 lg:col-span-2">
            <div class="flex items-center mb-6">
              <div class="w-10 h-10 bg-gradient-to-br from-emerald-500 to-teal-500 rounded-xl flex items-center justify-center mr-3">
                <svg class="w-5 h-5 text-white" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
                </svg>
              </div>
              <div>
                <h3 class="text-xl font-bold text-slate-800">Performance Settings</h3>
                <p class="text-slate-600 text-sm">Adjust typing speed and behavior</p>
              </div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div>
                <label for="typingInterval" class="block text-slate-700 font-medium mb-2">
                  Typing Speed
                </label>
                <div class="relative">
                  <input 
                    type="number" 
                    step="0.01" 
                    min="0.01"
                    max="1"
                    id="typingInterval" 
                    class="input-field pr-16" 
                    bind:value="{config.typingInterval}"
                  >
                  <div class="absolute inset-y-0 right-0 flex items-center pr-3">
                    <span class="text-xs text-slate-500">seconds</span>
                  </div>
                </div>
                <p class="text-xs text-slate-500 mt-1">Time between each character when typing (0.01 - 1.0)</p>
              </div>
              
              <div class="flex items-center justify-center">
                <div class="text-center">
                  <div class="text-2xl font-bold text-slate-800 mb-1">
                    {Math.round(1 / (config.typingInterval || 0.05))} 
                  </div>
                  <div class="text-sm text-slate-600">characters/sec</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Save Button -->
        <div class="mt-8 flex justify-center">
          <button 
            class="btn-primary group"
            on:click="{handleSaveConfig}"
          >
            <svg class="w-5 h-5 mr-2 group-hover:scale-110 transition-transform" fill="currentColor" viewBox="0 0 24 24">
              <path d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
            </svg>
            Save Configuration
          </button>
        </div>
      </section>
    {/if}
  </div>
</main>
