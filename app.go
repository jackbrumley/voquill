package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"runtime"
	"time"

	"gopkg.in/ini.v1"
)

// App struct
type App struct {
	ctx context.Context
}

// Application state
type AppState struct {
	config        Config
	tempAudioFile string
	isRecording   bool
	stopRecording chan bool
	history       []TranscriptionEntry
}

// Config represents the application configuration
type Config struct {
	APIKey         string
	Hotkey         string
	TypingInterval time.Duration
}

// TranscriptionEntry represents a single transcription in history
type TranscriptionEntry struct {
	Timestamp time.Time `json:"timestamp"`
	Text      string    `json:"text"`
	Duration  float64   `json:"duration"`
}

const (
	whisperAPIURL    = "https://api.openai.com/v1/audio/transcriptions"
	sampleRate       = 16000
	installedVersion = "1.0.0"
)

var appState *AppState

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{}
}

// startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
	// Initialize application state
	appState = &AppState{
		config: Config{
			APIKey:         "",
			Hotkey:         "Ctrl+Shift+R",
			TypingInterval: 50 * time.Millisecond,
		},
	}

	// Load configuration
	if err := loadConfig(); err != nil {
		fmt.Printf("Configuration error: %v\n", err)
	}

	// Load history
	loadHistory()

	// Set up temp file path
	appState.tempAudioFile = filepath.Join(os.TempDir(), "voquill_temp.wav")
}

// Greet returns a greeting for the given name
func (a *App) Greet(name string) string {
	return fmt.Sprintf("Hello %s, It's show time!", name)
}

// RecordAndTranscribe handles the complete recording and transcription process
func (a *App) RecordAndTranscribe() string {
	if appState.isRecording {
		return "Already recording"
	}

	appState.isRecording = true
	appState.stopRecording = make(chan bool, 1)

	defer func() {
		appState.isRecording = false
	}()

	fmt.Println("Starting recording...")

	startTime := time.Now()
	err := recordWavInterruptible(appState.tempAudioFile, appState.stopRecording)
	if err != nil {
		fmt.Printf("Recording error: %v\n", err)
		return fmt.Sprintf("Recording error: %v", err)
	}

	fmt.Println("Transcribing...")

	text, err := transcribeWhisper(appState.tempAudioFile)
	if err != nil {
		fmt.Printf("Transcription error: %v\n", err)
		return fmt.Sprintf("Transcription error: %v", err)
	}

	if text != "" {
		fmt.Printf("Typing: %s\n", text)
		simulateTyping(text)

		duration := time.Since(startTime).Seconds()
		addToHistory(text, duration)
	}

	os.Remove(appState.tempAudioFile)
	return text
}

// StopRecording stops the recording
func (a *App) StopRecording() {
	if appState.isRecording {
		appState.stopRecording <- true
	}
}

// GetHistory returns the transcription history as a slice of maps
func (a *App) GetHistory() []map[string]interface{} {
	var history []map[string]interface{}
	for _, entry := range appState.history {
		history = append(history, map[string]interface{}{
			"Text":      entry.Text,
			"Timestamp": entry.Timestamp,
			"Duration":  entry.Duration,
		})
	}
	return history
}

// GetConfig returns the current configuration as a map
func (a *App) GetConfig() map[string]interface{} {
	return map[string]interface{}{
		"apiKey":         appState.config.APIKey,
		"hotkey":         appState.config.Hotkey,
		"typingInterval": appState.config.TypingInterval.Seconds(),
	}
}

// SaveConfig saves the configuration from a map
func (a *App) SaveConfig(config map[string]interface{}) {
	if apiKey, ok := config["apiKey"].(string); ok {
		appState.config.APIKey = apiKey
	}
	if hotkey, ok := config["hotkey"].(string); ok {
		appState.config.Hotkey = hotkey
	}
	if typingInterval, ok := config["typingInterval"].(float64); ok {
		appState.config.TypingInterval = time.Duration(typingInterval * float64(time.Second))
	}

	if err := saveConfig(); err != nil {
		fmt.Printf("Error saving config: %v\n", err)
	}
}

// getConfigPath returns the OS-specific path for the config file
func getConfigPath() string {
	usr, _ := user.Current()
	base := usr.HomeDir
	if runtime.GOOS == "windows" {
		return filepath.Join(base, "AppData", "Local", "voquill", "config.ini")
	}
	return filepath.Join(base, ".config", "voquill", "config.ini")
}

// getHistoryPath returns the OS-specific path for the history file
func getHistoryPath() string {
	usr, _ := user.Current()
	base := usr.HomeDir
	if runtime.GOOS == "windows" {
		return filepath.Join(base, "AppData", "Local", "voquill", "history.json")
	}
	return filepath.Join(base, ".config", "voquill", "history.json")
}

// loadConfig loads and parses the configuration file
func loadConfig() error {
	cfgPath := getConfigPath()
	os.MkdirAll(filepath.Dir(cfgPath), 0755)

	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		file, _ := os.Create(cfgPath)
		defer file.Close()
		file.WriteString("WHISPER_API_KEY = your_api_key_here\nTYPING_SPEED_INTERVAL = 0.05\nHOTKEY = Ctrl+Shift+R\n")
		return fmt.Errorf("created new config file, please enter your OpenAI API key in: %s", cfgPath)
	}

	cfg, err := ini.Load(cfgPath)
	if err != nil {
		return err
	}

	appState.config.APIKey = cfg.Section("").Key("WHISPER_API_KEY").String()
	interval := cfg.Section("").Key("TYPING_SPEED_INTERVAL").MustFloat64(0.05)
	appState.config.TypingInterval = time.Duration(interval * float64(time.Second))
	appState.config.Hotkey = cfg.Section("").Key("HOTKEY").MustString("Ctrl+Shift+R")

	if appState.config.APIKey == "your_api_key_here" || appState.config.APIKey == "" {
		return fmt.Errorf("please edit your config file and enter a valid OpenAI API key: %s", cfgPath)
	}
	return nil
}

// saveConfig saves the current configuration
func saveConfig() error {
	cfgPath := getConfigPath()
	os.MkdirAll(filepath.Dir(cfgPath), 0755)
	
	cfg := ini.Empty()
	cfg.Section("").Key("WHISPER_API_KEY").SetValue(appState.config.APIKey)
	cfg.Section("").Key("TYPING_SPEED_INTERVAL").SetValue(fmt.Sprintf("%.3f", appState.config.TypingInterval.Seconds()))
	cfg.Section("").Key("HOTKEY").SetValue(appState.config.Hotkey)
	return cfg.SaveTo(cfgPath)
}

// loadHistory loads transcription history from file
func loadHistory() {
	historyPath := getHistoryPath()
	if _, err := os.Stat(historyPath); os.IsNotExist(err) {
		appState.history = []TranscriptionEntry{}
		return
	}

	data, err := os.ReadFile(historyPath)
	if err != nil {
		fmt.Printf("Error reading history: %v\n", err)
		appState.history = []TranscriptionEntry{}
		return
	}

	if err := json.Unmarshal(data, &appState.history); err != nil {
		fmt.Printf("Error parsing history: %v\n", err)
		appState.history = []TranscriptionEntry{}
	}
}

// saveHistory saves transcription history to file
func saveHistory() {
	historyPath := getHistoryPath()
	os.MkdirAll(filepath.Dir(historyPath), 0755)

	data, err := json.MarshalIndent(appState.history, "", "  ")
	if err != nil {
		fmt.Printf("Error marshaling history: %v\n", err)
		return
	}

	if err := os.WriteFile(historyPath, data, 0644); err != nil {
		fmt.Printf("Error saving history: %v\n", err)
	}
}

// addToHistory adds a new transcription to history
func addToHistory(text string, duration float64) {
	entry := TranscriptionEntry{
		Timestamp: time.Now(),
		Text:      text,
		Duration:  duration,
	}

	appState.history = append([]TranscriptionEntry{entry}, appState.history...)

	// Keep only last 100 entries
	if len(appState.history) > 100 {
		appState.history = appState.history[:100]
	}

	saveHistory()
}

// recordWavInterruptible records audio and saves it as a WAV file with stop channel
func recordWavInterruptible(filename string, stopChan chan bool) error {
	fmt.Println("Starting audio recording...")
	
	var cmd *exec.Cmd
	
	// Use platform-specific recording commands
	switch runtime.GOOS {
	case "linux":
		// Try different recording tools in order of preference
		if _, err := exec.LookPath("arecord"); err == nil {
			// Use ALSA arecord (most common on Linux)
			cmd = exec.Command("arecord", "-f", "S16_LE", "-r", "16000", "-c", "1", "-d", "30", filename)
		} else if _, err := exec.LookPath("sox"); err == nil {
			// Use SoX as fallback
			cmd = exec.Command("sox", "-t", "alsa", "default", "-r", "16000", "-c", "1", filename, "trim", "0", "30")
		} else {
			return fmt.Errorf("no audio recording tool found. Please install alsa-utils (arecord) or sox")
		}
	case "darwin":
		// Use macOS built-in recording
		cmd = exec.Command("sox", "-t", "coreaudio", "default", "-r", "16000", "-c", "1", filename, "trim", "0", "30")
	case "windows":
		// Use Windows built-in recording via PowerShell
		return fmt.Errorf("Windows audio recording not yet implemented")
	default:
		return fmt.Errorf("unsupported operating system: %s", runtime.GOOS)
	}
	
	// Start the recording process
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start recording: %v", err)
	}
	
	// Wait for stop signal or process completion
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()
	
	select {
	case <-stopChan:
		fmt.Println("Recording stopped by user")
		if cmd.Process != nil {
			cmd.Process.Kill()
		}
		<-done // Wait for process to actually stop
	case err := <-done:
		fmt.Println("Recording completed")
		if err != nil {
			return fmt.Errorf("recording process error: %v", err)
		}
	case <-time.After(30 * time.Second):
		fmt.Println("Recording stopped due to timeout")
		if cmd.Process != nil {
			cmd.Process.Kill()
		}
		<-done
	}
	
	return nil
}


// transcribeWhisper sends the audio file to OpenAI and returns the text
func transcribeWhisper(filename string) (string, error) {
	// Check if we have a valid API key
	if appState.config.APIKey == "" || appState.config.APIKey == "your_api_key_here" {
		return "MOCK: Please configure your OpenAI API key in settings to enable real transcription", nil
	}

	buf := new(bytes.Buffer)
	writer := multipart.NewWriter(buf)

	file, err := os.Open(filename)
	if err != nil {
		return "", err
	}
	defer file.Close()

	part, _ := writer.CreateFormFile("file", filepath.Base(filename))
	io.Copy(part, file)
	writer.WriteField("model", "whisper-1")
	writer.Close()

	req, _ := http.NewRequest("POST", whisperAPIURL, buf)
	req.Header.Set("Authorization", "Bearer "+appState.config.APIKey)
	req.Header.Set("Content-Type", writer.FormDataContentType())

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	var result struct {
		Text string `json:"text"`
	}
	json.NewDecoder(resp.Body).Decode(&result)
	return result.Text, nil
}

// simulateTyping simulates typing by sending keyboard events
func simulateTyping(text string) {
	for _, char := range text {
		typeCharacter(char)
		time.Sleep(appState.config.TypingInterval)
	}
}
