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

<main class="min-h-screen bg-gradient-to-br from-slate-50 to-blue-50">
  <div class="container mx-auto p-6 max-w-4xl">
    <header class="mb-8">
      <h1 class="text-4xl font-bold text-slate-800 dark:text-white mb-6 text-center">
        🎤 Voquill
      </h1>
      <nav class="flex justify-center space-x-2 bg-white dark:bg-slate-800 rounded-xl p-2 shadow-lg">
        <button 
          class="px-6 py-3 rounded-lg font-medium transition-all duration-200 {currentTab === 'status' ? 'bg-blue-500 text-white shadow-md' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}" 
          on:click="{() => currentTab = 'status'}"
        >
          📊 Status
        </button>
        <button 
          class="px-6 py-3 rounded-lg font-medium transition-all duration-200 {currentTab === 'history' ? 'bg-blue-500 text-white shadow-md' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}" 
          on:click="{() => currentTab = 'history'}"
        >
          📝 History
        </button>
        <button 
          class="px-6 py-3 rounded-lg font-medium transition-all duration-200 {currentTab === 'settings' ? 'bg-blue-500 text-white shadow-md' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}" 
          on:click="{() => currentTab = 'settings'}"
        >
          ⚙️ Settings
        </button>
      </nav>
    </header>

    {#if currentTab === 'status'}
      <section class="bg-white dark:bg-slate-800 rounded-2xl p-8 shadow-xl">
        <h2 class="text-2xl font-bold text-slate-800 dark:text-white mb-4">Recording Status</h2>
        <div class="text-center mb-8">
          {#if isRecording}
            <div class="inline-flex items-center px-6 py-3 rounded-full bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200 mb-6">
              <div class="w-3 h-3 bg-red-500 rounded-full mr-3 animate-pulse"></div>
              🔴 Recording in progress...
            </div>
          {:else if isTranscribing}
            <div class="inline-flex items-center px-6 py-3 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 mb-6">
              <div class="w-3 h-3 bg-blue-500 rounded-full mr-3 animate-pulse"></div>
              🧠 Transcribing audio...
            </div>
          {:else if isTyping}
            <div class="inline-flex items-center px-6 py-3 rounded-full bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200 mb-6">
              <div class="w-3 h-3 bg-purple-500 rounded-full mr-3 animate-pulse"></div>
              ⌨️ Typing text...
            </div>
          {:else if hotkeyPressed}
            <div class="inline-flex items-center px-6 py-3 rounded-full bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200 mb-6">
              <div class="w-3 h-3 bg-yellow-500 rounded-full mr-3 animate-pulse"></div>
              🎯 Hotkey detected - ready to record
            </div>
          {:else}
            <div class="inline-flex items-center px-6 py-3 rounded-full bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 mb-6">
              <div class="w-3 h-3 bg-green-500 rounded-full mr-3 animate-pulse"></div>
              ✅ Ready for dictation
            </div>
          {/if}
        </div>
        
        <div class="flex flex-col items-center space-y-6">
          <div class="bg-blue-50 dark:bg-slate-700 rounded-lg p-6 max-w-lg">
            <h3 class="text-lg font-semibold text-slate-800 dark:text-slate-200 mb-3 text-center">
              🎤 How to Use Voquill
            </h3>
            <div class="space-y-2 text-slate-600 dark:text-slate-300">
              <p class="flex items-center">
                <span class="text-blue-500 mr-2">1.</span>
                Position your cursor where you want text to appear
              </p>
              <p class="flex items-center">
                <span class="text-blue-500 mr-2">2.</span>
                Press and hold <kbd class="bg-slate-200 dark:bg-slate-600 px-2 py-1 rounded text-sm font-mono">{config.hotkey}</kbd>
              </p>
              <p class="flex items-center">
                <span class="text-blue-500 mr-2">3.</span>
                Speak your message clearly
              </p>
              <p class="flex items-center">
                <span class="text-blue-500 mr-2">4.</span>
                Release the hotkey to stop and transcribe
              </p>
            </div>
          </div>
        </div>
      </section>
    {:else if currentTab === 'history'}
      <section class="bg-white dark:bg-slate-800 rounded-2xl p-8 shadow-xl">
        <div class="flex justify-between items-center mb-6">
          <h2 class="text-2xl font-bold text-slate-800 dark:text-white">Transcription History</h2>
          <button class="bg-slate-100 dark:bg-slate-700 hover:bg-slate-200 dark:hover:bg-slate-600 text-slate-600 dark:text-slate-300 px-4 py-2 rounded-lg transition-colors duration-200">
            🗑️ Clear All
          </button>
        </div>
        
        <div class="space-y-4 max-h-96 overflow-y-auto">
          {#each history as entry, index}
            <div class="bg-slate-50 dark:bg-slate-700 rounded-lg p-4 hover:shadow-md transition-shadow duration-200">
              <div class="flex justify-between items-start mb-2">
                <p class="text-slate-500 dark:text-slate-400 text-sm font-medium">
                  {new Date(entry.Timestamp).toLocaleString()}
                </p>
                <button 
                  class="bg-blue-100 dark:bg-blue-900 hover:bg-blue-200 dark:hover:bg-blue-800 text-blue-600 dark:text-blue-300 px-3 py-1 rounded text-sm transition-colors duration-200"
                  on:click="{() => navigator.clipboard.writeText(entry.Text)}"
                >
                  📋 Copy
                </button>
              </div>
              <p class="text-slate-800 dark:text-slate-200 leading-relaxed">{entry.Text}</p>
            </div>
          {/each}
        </div>
      </section>
    {:else if currentTab === 'settings'}
      <section class="bg-white dark:bg-slate-800 rounded-2xl p-8 shadow-xl">
        <h2 class="text-2xl font-bold text-slate-800 dark:text-white mb-6">Settings</h2>
        
        <div class="space-y-6">
          <div>
            <label for="apiKey" class="block text-slate-700 dark:text-slate-300 font-medium mb-2">
              🔑 OpenAI API Key
            </label>
            <input 
              type="password" 
              id="apiKey" 
              class="w-full bg-slate-50 dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded-lg px-4 py-3 text-slate-800 dark:text-slate-200 focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200" 
              placeholder="sk-..."
              bind:value="{config.apiKey}"
            >
          </div>
          
          <div>
            <label for="hotkey" class="block text-slate-700 dark:text-slate-300 font-medium mb-2">
              ⌨️ Global Hotkey
            </label>
            <input 
              type="text" 
              id="hotkey" 
              class="w-full bg-slate-50 dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded-lg px-4 py-3 text-slate-800 dark:text-slate-200 focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200" 
              placeholder="Ctrl+Shift+Alt"
              bind:value="{config.hotkey}"
            >
          </div>
          
          <div>
            <label for="typingInterval" class="block text-slate-700 dark:text-slate-300 font-medium mb-2">
              ⚡ Typing Speed (seconds between characters)
            </label>
            <input 
              type="number" 
              step="0.01" 
              id="typingInterval" 
              class="w-full bg-slate-50 dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded-lg px-4 py-3 text-slate-800 dark:text-slate-200 focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200" 
              bind:value="{config.typingInterval}"
            >
          </div>
          
          <button 
            class="bg-green-500 hover:bg-green-600 text-white font-bold py-3 px-6 rounded-lg shadow-lg transform hover:scale-105 transition-all duration-200 flex items-center space-x-2" 
            on:click="{handleSaveConfig}"
          >
            <span>💾</span>
            <span>Save Settings</span>
          </button>
        </div>
      </section>
    {/if}
  </div>
</main>
