package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"
	"time"
	"fyne.io/fyne/v2"
)

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
