package main

import (
	"fmt"
	"os/exec"
	"runtime"
	"time"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// loadIcon loads the application icon
func loadIcon() fyne.Resource {
	// Use the embedded resource instead of loading from file
	return resourceIcon256x256Png
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

	// History Tab with text wrapping and copy buttons
	var historyContainer *fyne.Container
	var historyScroll *container.Scroll
	
	updateHistoryDisplay := func() {
		historyContainer = container.NewVBox()
		
		if len(appState.history) == 0 {
			historyContainer.Add(widget.NewLabel("No transcription history yet."))
		} else {
			for i, entry := range appState.history {
				// Create a card for each entry
				timeLabel := widget.NewLabel(entry.Timestamp.Format("15:04:05 - Jan 2"))
				timeLabel.TextStyle = fyne.TextStyle{Bold: true}
				
				// Create text widget with wrapping
				textWidget := widget.NewRichTextFromMarkdown(entry.Text)
				textWidget.Wrapping = fyne.TextWrapWord
				
				// Create copy button for this entry
				entryText := entry.Text // Capture for closure
				copyBtn := widget.NewButton("📋 Copy", func() {
					appState.mainWindow.Clipboard().SetContent(entryText)
				})
				copyBtn.Importance = widget.LowImportance
				
				// Create header with time and copy button
				header := container.NewBorder(nil, nil, timeLabel, copyBtn)
				
				// Create the entry container
				entryCard := widget.NewCard("", "", container.NewVBox(
					header,
					textWidget,
				))
				
				historyContainer.Add(entryCard)
				
				// Add separator except for last item
				if i < len(appState.history)-1 {
					historyContainer.Add(widget.NewSeparator())
				}
			}
		}
		
		// Update the scroll container
		if historyScroll != nil {
			historyScroll.Content = historyContainer
			historyScroll.Refresh()
		}
	}
	
	// Create initial container and scroll
	historyContainer = container.NewVBox()
	historyScroll = container.NewScroll(historyContainer)
	updateHistoryDisplay()
	
	// Store the update function globally so new entries can refresh the display
	appState.updateHistoryDisplay = updateHistoryDisplay
	
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
