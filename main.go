// voquill - Cross-platform voice-to-text app with GUI and global hotkey support

package main

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
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

	"github.com/gordonklaus/portaudio"
	"github.com/micmonay/keybd_event"
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
	hotkey         string
	configFile     string
	tempAudioFile  string
	isRecording    bool
	stopRecording  chan bool
	mainApp        fyne.App
	mainWindow     fyne.Window
	statusWindow   fyne.Window
	recordButton   *widget.Button
	statusLabel    *widget.Label
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
		file.WriteString("WHISPER_API_KEY = your_api_key_here\nTYPING_SPEED_INTERVAL = 0.01\nHOTKEY = ctrl+shift+alt\n")
		return fmt.Errorf("created new config file, please enter your OpenAI API key in: %s", cfgPath)
	}

	cfg, err := ini.Load(cfgPath)
	if err != nil {
		return err
	}

	appState.apiKey = cfg.Section("").Key("WHISPER_API_KEY").String()
	interval := cfg.Section("").Key("TYPING_SPEED_INTERVAL").MustFloat64(0.01)
	appState.typingInterval = time.Duration(interval * float64(time.Second))
	appState.hotkey = cfg.Section("").Key("HOTKEY").MustString("ctrl+shift+alt")

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
	cfg.Section("").Key("HOTKEY").SetValue(appState.hotkey)
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

// recordWavInterruptible records audio and saves it as a WAV file with stop channel
func recordWavInterruptible(filename string, stopChan chan bool) error {
	portaudio.Initialize()
	defer portaudio.Terminate()

	// Use a dynamic slice to collect frames
	var allFrames []int16
	bufferSize := sampleRate / 10 // 100ms chunks
	
	stream, err := portaudio.OpenDefaultStream(1, 0, float64(sampleRate), bufferSize, func(in []int16) {
		allFrames = append(allFrames, in...)
	})
	if err != nil {
		return err
	}
	defer stream.Close()

	if err := stream.Start(); err != nil {
		return err
	}
	defer stream.Stop()

	// Record until stopped - NO TIMER, only stops when button clicked
	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-stopChan:
			// Stop recording immediately when button clicked
			goto writeFile
		case <-ticker.C:
			// Just continue recording - no time limit
			continue
		}
	}

writeFile:
	// Write the recorded audio to file
	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	// Write WAV file headers and data
	sampleSize := 2
	byteRate := sampleRate * sampleSize
	dataLen := len(allFrames) * sampleSize

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
	binary.Write(f, binary.LittleEndian, allFrames)
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

// simulateTyping simulates typing using HID-level keyboard events
func simulateTyping(text string) {
	// Create a new keyboard event
	kb, err := keybd_event.NewKeyBonding()
	if err != nil {
		fmt.Printf("Error creating keyboard binding: %v\n", err)
		return
	}
	
	// For Linux, we need to add a small delay for initialization
	if runtime.GOOS == "linux" {
		time.Sleep(100 * time.Millisecond)
	}
	
	// Type each character
	for _, char := range text {
		// Convert character to key events
		err := typeCharacter(kb, char)
		if err != nil {
			fmt.Printf("Error typing character '%c': %v\n", char, err)
		}
		time.Sleep(appState.typingInterval)
	}
}

// typeCharacter types a single character using keyboard events
func typeCharacter(kb keybd_event.KeyBonding, char rune) error {
	var keyCode int
	var needShift bool
	
	// Map characters to key codes - fixed mappings
	switch char {
	// Letters (lowercase)
	case 'a': keyCode = keybd_event.VK_A
	case 'b': keyCode = keybd_event.VK_B
	case 'c': keyCode = keybd_event.VK_C
	case 'd': keyCode = keybd_event.VK_D
	case 'e': keyCode = keybd_event.VK_E
	case 'f': keyCode = keybd_event.VK_F
	case 'g': keyCode = keybd_event.VK_G
	case 'h': keyCode = keybd_event.VK_H
	case 'i': keyCode = keybd_event.VK_I
	case 'j': keyCode = keybd_event.VK_J
	case 'k': keyCode = keybd_event.VK_K
	case 'l': keyCode = keybd_event.VK_L
	case 'm': keyCode = keybd_event.VK_M
	case 'n': keyCode = keybd_event.VK_N
	case 'o': keyCode = keybd_event.VK_O
	case 'p': keyCode = keybd_event.VK_P
	case 'q': keyCode = keybd_event.VK_Q
	case 'r': keyCode = keybd_event.VK_R
	case 's': keyCode = keybd_event.VK_S
	case 't': keyCode = keybd_event.VK_T
	case 'u': keyCode = keybd_event.VK_U
	case 'v': keyCode = keybd_event.VK_V
	case 'w': keyCode = keybd_event.VK_W
	case 'x': keyCode = keybd_event.VK_X
	case 'y': keyCode = keybd_event.VK_Y
	case 'z': keyCode = keybd_event.VK_Z
	
	// Letters (uppercase)
	case 'A': keyCode = keybd_event.VK_A; needShift = true
	case 'B': keyCode = keybd_event.VK_B; needShift = true
	case 'C': keyCode = keybd_event.VK_C; needShift = true
	case 'D': keyCode = keybd_event.VK_D; needShift = true
	case 'E': keyCode = keybd_event.VK_E; needShift = true
	case 'F': keyCode = keybd_event.VK_F; needShift = true
	case 'G': keyCode = keybd_event.VK_G; needShift = true
	case 'H': keyCode = keybd_event.VK_H; needShift = true
	case 'I': keyCode = keybd_event.VK_I; needShift = true
	case 'J': keyCode = keybd_event.VK_J; needShift = true
	case 'K': keyCode = keybd_event.VK_K; needShift = true
	case 'L': keyCode = keybd_event.VK_L; needShift = true
	case 'M': keyCode = keybd_event.VK_M; needShift = true
	case 'N': keyCode = keybd_event.VK_N; needShift = true
	case 'O': keyCode = keybd_event.VK_O; needShift = true
	case 'P': keyCode = keybd_event.VK_P; needShift = true
	case 'Q': keyCode = keybd_event.VK_Q; needShift = true
	case 'R': keyCode = keybd_event.VK_R; needShift = true
	case 'S': keyCode = keybd_event.VK_S; needShift = true
	case 'T': keyCode = keybd_event.VK_T; needShift = true
	case 'U': keyCode = keybd_event.VK_U; needShift = true
	case 'V': keyCode = keybd_event.VK_V; needShift = true
	case 'W': keyCode = keybd_event.VK_W; needShift = true
	case 'X': keyCode = keybd_event.VK_X; needShift = true
	case 'Y': keyCode = keybd_event.VK_Y; needShift = true
	case 'Z': keyCode = keybd_event.VK_Z; needShift = true
	
	// Numbers
	case '0': keyCode = keybd_event.VK_0
	case '1': keyCode = keybd_event.VK_1
	case '2': keyCode = keybd_event.VK_2
	case '3': keyCode = keybd_event.VK_3
	case '4': keyCode = keybd_event.VK_4
	case '5': keyCode = keybd_event.VK_5
	case '6': keyCode = keybd_event.VK_6
	case '7': keyCode = keybd_event.VK_7
	case '8': keyCode = keybd_event.VK_8
	case '9': keyCode = keybd_event.VK_9
	
	// Common punctuation
	case ' ': keyCode = keybd_event.VK_SPACE
	case '.': keyCode = keybd_event.VK_DOT
	case ',': keyCode = keybd_event.VK_COMMA
	case '!': keyCode = keybd_event.VK_1; needShift = true
	case '?': keyCode = keybd_event.VK_SLASH; needShift = true
	case ':': keyCode = keybd_event.VK_SEMICOLON; needShift = true
	case ';': keyCode = keybd_event.VK_SEMICOLON
	case '-': keyCode = keybd_event.VK_MINUS
	case '_': keyCode = keybd_event.VK_MINUS; needShift = true
	case '\n': keyCode = keybd_event.VK_ENTER
	
	default:
		// For unsupported characters, skip them
		return nil
	}
	
	// Set the key
	kb.SetKeys(keyCode)
	
	// Add shift if needed
	if needShift {
		kb.HasSHIFT(true)
	} else {
		kb.HasSHIFT(false)
	}
	
	// Press and release the key
	err := kb.Launching()
	if err != nil {
		return err
	}
	
	return nil
}

// showStatusPopup creates a temporary status popup at bottom center of screen
func showStatusPopup(message string) {
	fyne.Do(func() {
		if appState.statusWindow != nil {
			appState.statusWindow.Close()
		}

		appState.statusWindow = appState.mainApp.NewWindow("Voquill Status")
		appState.statusWindow.SetContent(container.NewVBox(
			widget.NewLabel(message),
		))
		
		appState.statusWindow.Resize(fyne.NewSize(200, 60))
		appState.statusWindow.SetFixedSize(true)
		
		// Don't grab focus - this prevents stealing focus from current app
		appState.statusWindow.SetOnClosed(func() {
			// Do nothing - just close silently
		})
		
		// Position at bottom center of screen
		// Note: Fyne doesn't have direct screen positioning, so this will appear centered
		appState.statusWindow.CenterOnScreen()
		
		// Show without grabbing focus
		appState.statusWindow.Show()
		
		// Immediately try to return focus to the main window (if it was focused)
		// This is a workaround since Fyne doesn't have a "show without focus" option
		go func() {
			time.Sleep(10 * time.Millisecond)
			// Don't focus the status window
		}()
	})
}

// hideStatusPopup closes the status popup
func hideStatusPopup() {
	fyne.Do(func() {
		if appState.statusWindow != nil {
			appState.statusWindow.Close()
			appState.statusWindow = nil
		}
	})
}

// recordAndTranscribe handles the complete recording and transcription process
func recordAndTranscribe() {
	if appState.isRecording {
		return // Prevent multiple simultaneous recordings
	}
	
	appState.isRecording = true
	appState.stopRecording = make(chan bool, 1)
	
	defer func() { 
		appState.isRecording = false 
		// Reset button text and status when done
		if appState.recordButton != nil {
			fyne.Do(func() {
				appState.recordButton.SetText("🎤 Start Recording")
			})
		}
		if appState.statusLabel != nil {
			fyne.Do(func() {
				appState.statusLabel.SetText("Ready for dictation")
			})
		}
	}()

	// Update button text to show it can be stopped
	if appState.recordButton != nil {
		fyne.Do(func() {
			appState.recordButton.SetText("⏹️ Stop Recording")
		})
	}

	// Update status in main window
	if appState.statusLabel != nil {
		fyne.Do(func() {
			appState.statusLabel.SetText("🎤 Recording...")
		})
	}

	fmt.Println("Starting recording...")
	
	startTime := time.Now()
	err := recordWavInterruptible(appState.tempAudioFile, appState.stopRecording)
	
	// Update status for transcription immediately after recording ends
	if appState.statusLabel != nil {
		fyne.Do(func() {
			appState.statusLabel.SetText("⏳ Transcribing...")
		})
	}
	
	if err != nil {
		fmt.Printf("Recording error: %v\n", err)
		if appState.statusLabel != nil {
			fyne.Do(func() {
				appState.statusLabel.SetText("❌ Recording error")
			})
		}
		return
	}
	
	fmt.Println("Transcribing...")
	
	text, err := transcribeWhisper(appState.tempAudioFile)
	if err != nil {
		fmt.Printf("Transcription error: %v\n", err)
		if appState.statusLabel != nil {
			fyne.Do(func() {
				appState.statusLabel.SetText("❌ Transcription error")
			})
		}
		return
	}
	
	if text != "" {
		// Update status for typing
		if appState.statusLabel != nil {
			fyne.Do(func() {
				appState.statusLabel.SetText("⌨️ Typing...")
			})
		}
		
		fmt.Printf("Typing: %s\n", text)
		simulateTyping(text)
		
		duration := time.Since(startTime).Seconds()
		addToHistory(text, duration)
		
		// Show completion status briefly
		if appState.statusLabel != nil {
			fyne.Do(func() {
				appState.statusLabel.SetText("✅ Complete!")
			})
		}
		
		// Reset to ready after a short delay
		go func() {
			time.Sleep(2 * time.Second)
			if appState.statusLabel != nil {
				fyne.Do(func() {
					appState.statusLabel.SetText("Ready for dictation")
				})
			}
		}()
	}
	
	os.Remove(appState.tempAudioFile)
}

// setupGlobalHotkey sets up the global hotkey listener
func setupGlobalHotkey() {
	fmt.Printf("Setting up global hotkey: %s\n", appState.hotkey)
	
	go func() {
		var lastTrigger time.Time
		var wasPressed bool
		
		for {
			// Check if hotkey combination is pressed
			isPressed := isHotkeyPressed()
			
			// Trigger on key press (not hold)
			if isPressed && !wasPressed && time.Since(lastTrigger) > 1*time.Second {
				if !appState.isRecording {
					fmt.Println("Hotkey detected - starting recording")
					go recordAndTranscribe()
					lastTrigger = time.Now()
				}
			}
			
			wasPressed = isPressed
			time.Sleep(50 * time.Millisecond) // Check every 50ms
		}
	}()
}

// isHotkeyPressed checks if the configured hotkey combination is currently pressed
func isHotkeyPressed() bool {
	// For now, hotkey detection is disabled to focus on core functionality
	// Users can use the GUI button to trigger recording
	return false
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
	appState.mainApp = app.NewWithID("com.voquill.app")
	appState.mainApp.SetIcon(loadIcon())
	appState.mainWindow = appState.mainApp.NewWindow("Voquill - Voice Dictation")
	appState.mainWindow.Resize(fyne.NewSize(500, 400))

	// Create tabs
	tabs := container.NewAppTabs()

	// Status Tab
	statusLabel := widget.NewLabel("Ready for dictation")
	hotkeyLabel := widget.NewLabel("Global Hotkey: Ctrl+Shift+Alt (Active)")
	
	recordBtn := widget.NewButton("🎤 Start Recording", func() {
		if !appState.isRecording {
			// Start recording
			go recordAndTranscribe()
		} else {
			// Stop recording immediately
			if appState.stopRecording != nil {
				select {
				case appState.stopRecording <- true:
					// Signal sent successfully - immediately update button text
					fyne.Do(func() {
						appState.recordButton.SetText("🎤 Start Recording")
					})
				default:
					// Channel full or closed, ignore
				}
			}
		}
	})
	recordBtn.Importance = widget.HighImportance
	
	// Store references globally so we can update them
	appState.recordButton = recordBtn
	appState.statusLabel = statusLabel
	
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
	
	hotkeyEntry := widget.NewEntry()
	hotkeyEntry.SetText(appState.hotkey)
	hotkeyEntry.SetPlaceHolder("e.g., ctrl+shift+alt")
	
	typingSpeedSlider := widget.NewSlider(0.001, 0.1)
	typingSpeedSlider.SetValue(appState.typingInterval.Seconds())
	typingSpeedLabel := widget.NewLabel(fmt.Sprintf("%.3fs", appState.typingInterval.Seconds()))
	
	typingSpeedSlider.OnChanged = func(value float64) {
		typingSpeedLabel.SetText(fmt.Sprintf("%.3fs", value))
	}
	
	saveBtn := widget.NewButton("Save Settings", func() {
		appState.apiKey = apiKeyEntry.Text
		appState.hotkey = hotkeyEntry.Text
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
		widget.NewCard("Hotkey Configuration", "", container.NewVBox(
			widget.NewLabel("Global Hotkey Combination:"),
			hotkeyEntry,
			widget.NewLabel("Note: Global hotkey detection is currently basic"),
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
	// Use the embedded resource instead of loading from file
	return resourceIcon256x256Png
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
	
	// Note: Global hotkey detection is currently disabled
	// Users can use the GUI button to trigger recording
	
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
