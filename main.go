// voikey.go - Full cross-platform voice-to-text app with system tray, recording, OpenAI Whisper integration, and update checker

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

	"github.com/getlantern/systray"
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
	duration          = 5 * time.Second
	iconPath          = "assets/icon256x256.png"
)

var (
	apiKey         string
	typingInterval time.Duration
	configFile     string
	tempAudioFile  string
	statusWindow   fyne.Window
)

// getConfigPath returns the OS-specific path for the config file
func getConfigPath() string {
	usr, _ := user.Current()
	base := usr.HomeDir
	if runtime.GOOS == "windows" {
		return filepath.Join(base, "AppData", "Local", "voikey", "config.ini")
	}
	return filepath.Join(base, ".config", "voikey", "config.ini")
}

// loadConfig loads and parses the configuration file
func loadConfig() error {
	cfgPath := getConfigPath()
	configFile = cfgPath
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

	apiKey = cfg.Section("").Key("WHISPER_API_KEY").String()
	interval := cfg.Section("").Key("TYPING_SPEED_INTERVAL").MustFloat64(0.01)
	typingInterval = time.Duration(interval * float64(time.Second))

	if apiKey == "your_api_key_here" {
		return fmt.Errorf("please edit your config file and enter a valid OpenAI API key: %s", cfgPath)
	}
	return nil
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
	req.Header.Set("Authorization", "Bearer "+apiKey)
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
		time.Sleep(typingInterval)
	}
}

// loadIcon loads the tray icon from assets
func loadIcon() []byte {
	file, err := os.Open(iconPath)
	if err != nil {
		log.Println("Failed to open icon:", err)
		return nil
	}
	defer file.Close()
	img, _, err := image.Decode(file)
	if err != nil {
		log.Println("Failed to decode icon image:", err)
		return nil
	}
	buf := new(bytes.Buffer)
	if err := png.Encode(buf, img); err != nil {
		log.Println("Failed to encode PNG:", err)
		return nil
	}
	return buf.Bytes()
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
		log.Printf("Voikey is up to date. (v%s)\n", installedVersion)
	}
}

// openConfig opens the config file in the default editor
func openConfig() {
	var cmd *exec.Cmd
	if runtime.GOOS == "windows" {
		cmd = exec.Command("notepad", configFile)
	} else {
		cmd = exec.Command("xdg-open", configFile)
	}
	cmd.Start()
}

// showStatus creates a temporary popup status window
func showStatus(title, message string, delay time.Duration) {
	go func() {
		app := app.New()
		statusWindow = app.NewWindow(title)
		statusWindow.SetContent(container.NewVBox(widget.NewLabel(message)))
		statusWindow.Resize(fyne.NewSize(300, 100))
		statusWindow.Show()
		time.Sleep(delay)
		statusWindow.Close()
	}()
}

// recordAndTranscribe handles recording, transcription, and typing
func recordAndTranscribe() {
	fmt.Println("Recording audio...")
	showStatus("Voikey", "Recording...", duration+1*time.Second)
	recordWav(tempAudioFile, duration)
	fmt.Println("Transcribing...")
	showStatus("Voikey", "Transcribing...", 4*time.Second)
	text, err := transcribeWhisper(tempAudioFile)
	if err != nil {
		fmt.Println("Error:", err)
		return
	}
	fmt.Println("Typing: ", text)
	simulateTyping(text)
	os.Remove(tempAudioFile)
}

// onReady initializes the tray menu
func onReady() {
	systray.SetIcon(loadIcon())
	systray.SetTitle("Voikey")
	systray.SetTooltip("Voikey - Voice Typing")

	edCfg := systray.AddMenuItem("Edit Config", "Edit Configuration File")
	record := systray.AddMenuItem("Start Dictation", "Record & Transcribe")
	exit := systray.AddMenuItem("Quit", "Exit Application")

	go func() {
		for {
			select {
			case <-edCfg.ClickedCh:
				openConfig()
			case <-record.ClickedCh:
				recordAndTranscribe()
			case <-exit.ClickedCh:
				systray.Quit()
				return
			}
		}
	}()
	checkForUpdates()
	fmt.Println("Voikey is running.")
}

// main is the entry point
func main() {
	if err := loadConfig(); err != nil {
		fmt.Println("Error:", err)
		return
	}
	tempAudioFile = filepath.Join(os.TempDir(), "voikey_temp.wav")
	systray.Run(onReady, func() {})
}
