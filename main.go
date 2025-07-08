// voquill - Cross-platform voice-to-text app with GUI and global hotkey support

package main

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"image"
	"image/png"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/go-vgo/robotgo"
	"github.com/gordonklaus/portaudio"
	"gopkg.in/ini.v1"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

const (
	whisperAPIURL     = "https://api.openai.com/v1/audio/transcriptions"
	updateCheckURL    = "https://raw.githubusercontent.com/jackbrumley/voquill/main/version.txt"
	installedVersion  = "1.0.0"
	sampleRate        = 16000
	recordingDuration = 5 * time.Second
	iconPath          = "assets/icon256x256.png"
)

// Application state
type AppState struct {
	apiKey         string
	typingInterval time.Duration
	configFile     string
	tempAudioFile  string
	isRecording    bool
	mainApp        fyne.App
	mainWindow     fyne.Window
	statusWindow   fyne.Window
	history        []TranscriptionEntry
}

// TranscriptionEntry represents a single transcription in history
type TranscriptionEntry struct {
	Timestamp time.Time `json:"timestamp"`
	Text      string    `json:"text"`
	Duration  float64   `json:"duration"`
}

var appState *AppState

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
	appState.configFile = cfgPath
	os.MkdirAll(filepath.Dir(cfgPath), 0755)

	if _, err := os.Stat(cfgPath); os.IsNotExist(err) {
		file, _ := os.Create(cfgPath)
		defer file.Close()
		file.WriteString("WHISPER_API_KEY = your_api_key_here\nTYPING_SPEED_INTERVAL = 0.01\n")
		return fmt.Errorf("created new config file, please enter your OpenAI API key in: %s", cfgPath)
	}

	cfg, err := ini.Load(cfgPath)
	if err != nil {
		return err
	}

	appState.apiKey = cfg.Section("").Key("WHISPER_API_KEY").String()
	interval := cfg.Section("").Key("TYPING_SPEED_INTERVAL").MustFloat64(0.01)
	appState.typingInterval = time.Duration(interval * float64(time.Second))

	if appState.apiKey == "your_api_key_here" || appState.apiKey == "" {
		return fmt.Errorf("please edit your config file and enter a valid OpenAI API key: %s", cfgPath)
	}
	return nil
}

// saveConfig saves the current configuration
func saveConfig() error {
	cfg := ini.Empty()
	cfg.Section("").Key("WHISPER_API_KEY").SetValue(appState.apiKey)
	cfg.Section("").Key("TYPING_SPEED_INTERVAL").SetValue(fmt.Sprintf("%.3f", appState.typingInterval.Seconds()))
	return cfg.SaveTo(appState.configFile)
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
		log.Printf("Error reading history: %v", err)
		appState.history = []TranscriptionEntry{}
		return
	}

	if err := json.Unmarshal(data, &appState.history); err != nil {
		log.Printf("Error parsing history: %v", err)
		appState.history = []TranscriptionEntry{}
	}
}

// saveHistory saves transcription history to file
func saveHistory() {
	historyPath := getHistoryPath()
	os.MkdirAll(filepath.Dir(historyPath), 0755)

	data, err := json.MarshalIndent(appState.history, "", "  ")
	if err != nil {
		log.Printf("Error marshaling history: %v", err)
		return
	}

	if err := os.WriteFile(historyPath, data, 0644); err != nil {
		log.Printf("Error saving history: %v", err)
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

// recordWav records audio and saves it as a WAV file
func recordWav(filename string, duration time.Duration) error {
	portaudio.Initialize()
	defer portaudio.Terminate()

	frames := make([]int16, sampleRate*int(duration.Seconds()))
	stream, err := portaudio.OpenDefaultStream(1, 0, float64(sampleRate), len(frames), &frames)
	if err != nil {
		return err
	}
	defer stream.Close()

	if err := stream.Start(); err != nil {
		return err
	}
	time.Sleep(duration)
	stream.Read()
	stream.Stop()

	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	// Write WAV file headers and data
	sampleSize := 2
	byteRate := sampleRate * sampleSize
	dataLen := len(frames) * sampleSize

	f.WriteString("RIFF")
	binary.Write(f, binary.LittleEndian, uint32(36+dataLen))
	f.WriteString("WAVEfmt ")
	binary.Write(f, binary.LittleEndian, uint32(16))
	binary.Write(f, binary.LittleEndian, uint16(1))
	binary.Write(f, binary.LittleEndian, uint16(1))
	binary.Write(f, binary.LittleEndian, uint32(sampleRate))
	binary.Write(f, binary.LittleEndian, uint32(byteRate))
	binary.Write(f, binary.LittleEndian, uint16(sampleSize))
	binary.Write(f, binary.LittleEndian, uint16(16))
	f.WriteString("data")
	binary.Write(f, binary.LittleEndian, uint32(dataLen))
	binary.Write(f, binary.LittleEndian, frames)
	return nil
}

// transcribeWhisper sends the audio file to OpenAI and returns the text
func transcribeWhisper(filename string) (string, error) {
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
	req.Header.Set("Authorization", "Bearer "+appState.apiKey)
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

// simulateTyping simulates typing the string with a delay
func simulateTyping(text string) {
	for _, char := range text {
		robotgo.TypeStr(string(char))
		time.Sleep(appState.typingInterval)
	}
}

// showStatusPopup creates a temporary status popup at bottom center of screen
func showStatusPopup(message string) {
	if appState.statusWindow != nil {
		appState.statusWindow.Close()
	}

	appState.statusWindow = appState.mainApp.NewWindow("Voquill Status")
	appState.statusWindow.SetContent(container.NewVBox(
		widget.NewLabel(message),
	))
	
	appState.statusWindow.Resize(fyne.NewSize(200, 60))
	appState.statusWindow.SetFixedSize(true)
	
	// Position at bottom center of screen
	// Note: Fyne doesn't have direct screen positioning, so this will appear centered
	appState.statusWindow.CenterOnScreen()
	
	appState.statusWindow.Show()
}

// hideStatusPopup closes the status popup
func hideStatusPopup() {
	if appState.statusWindow != nil {
		appState.statusWindow.Close()
		appState.statusWindow = nil
	}
}

// recordAndTranscribe handles the complete recording and transcription process
func recordAndTranscribe() {
	if appState.isRecording {
		return // Prevent multiple simultaneous recordings
	}
	
	appState.isRecording = true
	defer func() { appState.isRecording = false }()

	fmt.Println("Starting recording...")
	showStatusPopup("Recording...")
	
	startTime := time.Now()
	err := recordWav(appState.tempAudioFile, recordingDuration)
	if err != nil {
		fmt.Printf("Recording error: %v\n", err)
		hideStatusPopup()
		return
	}
	
	fmt.Println("Transcribing...")
	showStatusPopup("Transcribing...")
	
	text, err := transcribeWhisper(appState.tempAudioFile)
	if err != nil {
		fmt.Printf("Transcription error: %v\n", err)
		hideStatusPopup()
		return
	}
	
	hideStatusPopup()
	
	if text != "" {
		fmt.Printf("Typing: %s\n", text)
		simulateTyping(text)
		
		duration := time.Since(startTime).Seconds()
		addToHistory(text, duration)
	}
	
	os.Remove(appState.tempAudioFile)
}

// setupGlobalHotkey sets up the global hotkey listener
func setupGlobalHotkey() {
	fmt.Println("Global hotkey setup temporarily disabled - use GUI button for now")
	// TODO: Implement cross-platform global hotkey support
	// For now, users can use the GUI button to test functionality
}

// checkForUpdates compares installed version to online version
func checkForUpdates() {
	resp, err := http.Get(updateCheckURL)
	if err != nil {
		log.Println("Update check failed:", err)
		return
	}
	defer resp.Body.Close()
	buf := new(strings.Builder)
	_, err = io.Copy(buf, resp.Body)
	if err == nil && strings.TrimSpace(buf.String()) != installedVersion {
		log.Printf("New version available: %s (current: %s)\n", buf.String(), installedVersion)
	} else {
		log.Printf("Voquill is up to date. (v%s)\n", installedVersion)
	}
}

// openConfigFile opens the config file in the default editor
func openConfigFile() {
	var cmd *exec.Cmd
	if runtime.GOOS == "windows" {
		cmd = exec.Command("notepad", appState.configFile)
	} else {
		cmd = exec.Command("xdg-open", appState.configFile)
	}
	cmd.Start()
}

// createMainGUI creates the main application window with tabs
func createMainGUI() {
	appState.mainApp = app.New()
	appState.mainApp.SetIcon(loadIcon())
	appState.mainWindow = appState.mainApp.NewWindow("Voquill - Voice Dictation")
	appState.mainWindow.Resize(fyne.NewSize(500, 400))

	// Create tabs
	tabs := container.NewAppTabs()

	// Status Tab
	statusLabel := widget.NewLabel("Ready for dictation")
	hotkeyLabel := widget.NewLabel("Global Hotkey: Coming soon (Wayland-compatible)")
	
	recordBtn := widget.NewButton("🎤 Start Recording", func() {
		go recordAndTranscribe()
	})
	recordBtn.Importance = widget.HighImportance
	
	statusTab := container.NewVBox(
		widget.NewCard("Status", "", container.NewVBox(
			statusLabel,
			hotkeyLabel,
		)),
		widget.NewCard("Manual Recording", "Click to test voice dictation", container.NewVBox(
			recordBtn,
			widget.NewLabel("Position cursor where you want text, then click Record"),
		)),
	)
	tabs.Append(container.NewTabItem("Status", statusTab))

	// History Tab - simplified for now
	historyText := widget.NewRichTextFromMarkdown("No transcription history yet.")
	historyScroll := container.NewScroll(historyText)
	
	updateHistoryDisplay := func() {
		if len(appState.history) == 0 {
			historyText.ParseMarkdown("No transcription history yet.")
		} else {
			var content strings.Builder
			content.WriteString("# Transcription History\n\n")
			for _, entry := range appState.history {
				content.WriteString(fmt.Sprintf("**%s**: %s\n\n", 
					entry.Timestamp.Format("15:04:05"), entry.Text))
			}
			historyText.ParseMarkdown(content.String())
		}
	}
	
	// Update display initially
	updateHistoryDisplay()
	
	clearHistoryBtn := widget.NewButton("Clear History", func() {
		appState.history = []TranscriptionEntry{}
		saveHistory()
		updateHistoryDisplay()
	})
	
	historyTab := container.NewBorder(nil, clearHistoryBtn, nil, nil, historyScroll)
	tabs.Append(container.NewTabItem("History", historyTab))

	// Settings Tab
	apiKeyEntry := widget.NewPasswordEntry()
	apiKeyEntry.SetText(appState.apiKey)
	
	typingSpeedSlider := widget.NewSlider(0.001, 0.1)
	typingSpeedSlider.SetValue(appState.typingInterval.Seconds())
	typingSpeedLabel := widget.NewLabel(fmt.Sprintf("%.3fs", appState.typingInterval.Seconds()))
	
	typingSpeedSlider.OnChanged = func(value float64) {
		typingSpeedLabel.SetText(fmt.Sprintf("%.3fs", value))
	}
	
	saveBtn := widget.NewButton("Save Settings", func() {
		appState.apiKey = apiKeyEntry.Text
		appState.typingInterval = time.Duration(typingSpeedSlider.Value * float64(time.Second))
		
		if err := saveConfig(); err != nil {
			fmt.Printf("Error saving config: %v\n", err)
		} else {
			fmt.Println("Settings saved successfully")
		}
	})
	
	openConfigBtn := widget.NewButton("Open Config File", openConfigFile)
	
	settingsTab := container.NewVBox(
		widget.NewCard("API Configuration", "", container.NewVBox(
			widget.NewLabel("OpenAI API Key:"),
			apiKeyEntry,
		)),
		widget.NewCard("Typing Settings", "", container.NewVBox(
			widget.NewLabel("Typing Speed:"),
			typingSpeedSlider,
			typingSpeedLabel,
		)),
		container.NewHBox(saveBtn, openConfigBtn),
	)
	tabs.Append(container.NewTabItem("Settings", settingsTab))

	appState.mainWindow.SetContent(tabs)
	
	// Handle window close - minimize instead of quit
	appState.mainWindow.SetCloseIntercept(func() {
		appState.mainWindow.Hide()
	})
}

// loadIcon loads the application icon
func loadIcon() fyne.Resource {
	if _, err := os.Stat(iconPath); err != nil {
		return nil
	}
	
	file, err := os.Open(iconPath)
	if err != nil {
		return nil
	}
	defer file.Close()
	
	img, _, err := image.Decode(file)
	if err != nil {
		return nil
	}
	
	buf := new(bytes.Buffer)
	if err := png.Encode(buf, img); err != nil {
		return nil
	}
	
	return fyne.NewStaticResource("icon.png", buf.Bytes())
}

// main is the entry point
func main() {
	// Initialize application state
	appState = &AppState{}
	
	// Load configuration
	if err := loadConfig(); err != nil {
		fmt.Printf("Configuration error: %v\n", err)
		fmt.Println("Please configure the application through the GUI.")
	}
	
	// Load history
	loadHistory()
	
	// Set up temp file path
	appState.tempAudioFile = filepath.Join(os.TempDir(), "voquill_temp.wav")
	
	// Create main GUI
	createMainGUI()
	
	// Start global hotkey listener in background
	go setupGlobalHotkey()
	
	// Check for updates
	go checkForUpdates()
	
	fmt.Println("Voquill is running. Use Ctrl+Shift+Alt to start dictation.")
	
	// Show main window and run app
	appState.mainWindow.ShowAndRun()
}
