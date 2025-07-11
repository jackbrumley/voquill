package main

import (
	"fmt"
	"os"
	"os/user"
	"path/filepath"
	"runtime"
	"time"
	"gopkg.in/ini.v1"
)

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
