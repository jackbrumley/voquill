package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"time"
)

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
	
	// Update the history display if the function is available
	if appState.updateHistoryDisplay != nil {
		appState.updateHistoryDisplay()
	}
}
