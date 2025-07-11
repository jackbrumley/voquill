// voquill - Cross-platform voice-to-text app with GUI and global hotkey support

package main

import (
	"fmt"
	"os"
	"path/filepath"
)

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
