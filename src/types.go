package main

import (
	"time"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// Application state
type AppState struct {
	apiKey              string
	typingInterval      time.Duration
	hotkey              string
	configFile          string
	tempAudioFile       string
	isRecording         bool
	stopRecording       chan bool
	mainApp             fyne.App
	mainWindow          fyne.Window
	statusWindow        fyne.Window
	recordButton        *widget.Button
	statusLabel         *widget.Label
	history             []TranscriptionEntry
	updateHistoryDisplay func()
}

// TranscriptionEntry represents a single transcription in history
type TranscriptionEntry struct {
	Timestamp time.Time `json:"timestamp"`
	Text      string    `json:"text"`
	Duration  float64   `json:"duration"`
}

const (
	whisperAPIURL     = "https://api.openai.com/v1/audio/transcriptions"
	updateCheckURL    = "https://raw.githubusercontent.com/jackbrumley/voquill/main/version.txt"
	installedVersion  = "1.0.0"
	sampleRate        = 16000
	recordingDuration = 5 * time.Second
	iconPath          = "assets/icon256x256.png"
)

var appState *AppState
