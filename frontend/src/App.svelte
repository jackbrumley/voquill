<script>
  import { onMount } from 'svelte';
  import { Greet, RecordAndTranscribe, StopRecording, GetHistory, GetConfig, SaveConfig } from '../wailsjs/go/main/App';

  let isRecording = false;
  let history = [];
  let config = {
    apiKey: '',
    hotkey: 'Ctrl+Shift+R',
    typingInterval: 0.05
  };
  let currentTab = 'status';

  onMount(async () => {
    try {
      history = await GetHistory();
      config = await GetConfig();
      console.log("Greet test:", await Greet("Voquill"));
    } catch (error) {
      console.error("Error loading initial data:", error);
    }
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
          <div class="inline-flex items-center px-4 py-2 rounded-full bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 mb-6">
            <div class="w-2 h-2 bg-green-500 rounded-full mr-2 animate-pulse"></div>
            Ready for dictation
          </div>
        </div>
        
        <div class="flex flex-col items-center space-y-6">
          {#if isRecording}
            <button 
              class="bg-red-500 hover:bg-red-600 text-white font-bold py-4 px-8 rounded-full text-lg shadow-lg transform hover:scale-105 transition-all duration-200 flex items-center space-x-2" 
              on:click="{handleStop}"
            >
              <span class="text-2xl">⏹️</span>
              <span>Stop Recording</span>
            </button>
            <div class="flex items-center space-x-2 text-red-600 dark:text-red-400">
              <div class="w-3 h-3 bg-red-500 rounded-full animate-pulse"></div>
              <span class="font-medium">Recording in progress...</span>
            </div>
          {:else}
            <button 
              class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-4 px-8 rounded-full text-lg shadow-lg transform hover:scale-105 transition-all duration-200 flex items-center space-x-2" 
              on:click="{handleRecord}"
            >
              <span class="text-2xl">🎤</span>
              <span>Start Recording</span>
            </button>
          {/if}
          
          <div class="bg-blue-50 dark:bg-slate-700 rounded-lg p-4 max-w-md">
            <p class="text-slate-600 dark:text-slate-300 text-center">
              💡 Position your cursor where you want the text to appear, then click record.
            </p>
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
              placeholder="Ctrl+Shift+R"
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
