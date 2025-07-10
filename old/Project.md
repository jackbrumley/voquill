# Voice-to-Text Dictation using OpenAI Whisper

Voice-to-text dictation app for Windows using OpenAI's Whisper API.

![alt text](/assets/icon256x256.png)

I found that the built-in Windows 10/11 Voice Typing tool was decent but often struggled with accuracy. I wanted something better—something that could type using my voice in any text box across my system. It needed to be simple, affordable, and reliable. While commercial tools like Dragon Naturally Speaking are excellent, they can be prohibitively expensive.

OpenAI's Whisper model (same model used when you talk to ChatGPT using the mobile app), is available via their API and offers voice recognition that's incredibly accurate and comes at a very low cost. I came across a GitHub repo where someone had built a basic tool to send requestes via the API and found it so useful I forked it, fixed a few bugs and built upon it.

**You need an OpenAI API key and sufficient credit on your account for this to work!**

This application runs in your system tray, waiting for a trigger keypress (CTRL+SHIFT+ALT by default). It uses your default microphone to record voice input until you release the trigger keys. The recording is sent to OpenAI's Whisper model for transcription, and the text is either typed out character by character or pasted instantly where your cursor is active, based on your configuration.

## What It Does

1. Waits in the system tray for the configured shortcut key to be pressed.
2. Records audio using your default microphone.
3. Sends the audio to OpenAI's Whisper API for transcription.
4. Types or pastes the transcription into the active application based on the selected output mode.
5. Displays status windows for recording and transcribing, positioned dynamically based on user settings.

## Features

- **Configurable Output Modes**: Choose between "typed" (character-by-character simulation) or "instant" (essentailly copy/paste). Note: The "instant" mode is not currently implimented.
- **Custom Shortcut Key**: Allows you to configure the key combination for triggering the dictation process.
- **Status Window**: Displays recording and transcription progress with adjustable positioning.
- **Tray Icon Menu**: Edit configuration and exit the app easily from the system tray.


## Prerequisites:

1. **Windows** 
   - While this application may work on other oparating systems, it has only been tested on Windows 10/11. It was originally intended to replace Windows Voice Typing.
   - 'More info in on Microsoft Voice Typing  [here](https://support.microsoft.com/en-au/windows/use-voice-typing-to-talk-instead-of-type-on-your-pc-fec94565-c4bd-329d-e59a-af033fa5689f).
2. **Python**
   - Download and install Python from the [official Python website](https://www.python.org/downloads/). Make sure to add Python to your system's PATH during installation.
   - Verify that Python is installed by running the following command in your terminal or command prompt: 
   `python --version`
3. **Python Packages**
   - To install the required Python packages, run the following command:
   **Note:** If you encounter any permissions issues, you may need to run this command as an administrator.
   - `pip install pyaudio keyboard requests pyautogui screeninfo pillow pystray`

## Download and Run

### Download
To download the latest version of the application, click [here](https://github.com/jackbrumley/whisper-dictation/archive/refs/heads/main.zip) to download a ZIP file of the repository.

1. Extract the contents of the ZIP file to a folder on your computer.
2. Navigate to the extracted folder and double-click `main.py`.


### Or  Clone

Alternatively if you have git installed you can you can clone the respository, browse to the directory and run it from the command line.
- `mkdir c:\Temp`
- `cd c:\Temp`
- `git clone https://github.com/jackbrumley/whisper-dictation`
- `cd c:\Temp\whisper-dictation`
- `python main.py`

## Configuration File

The configuration file (`config.ini`) is created in the user's AppData directory on the first run of the script. You can also edit the configuration directly through the "Edit Config" option in the system tray menu.

Path:
`C:\Users\<user>\AppData\Local\whisper-dictation\config.ini`

### Configuration Options

Below are the options available in the configuration file:

- `WHISPER_API_KEY` Your OpenAI API key (you must replace the placeholder key).
- `PIXELS_FROM_BOTTOM` Adjusts how far the status windows are positioned from the bottom of the screen (default: `80`).
- `KEYBOARD_SHORTCUT` The keyboard combination used to start recording (default: `ctrl+shift+alt`).
- `TYPING_SPEED_INTERVAL` The delay (in seconds) between each character when outputting transcribed text in "typed" mode. Adjust as needed for performance.
- `OUTPUT_MODE` Determines how the text is output:  
  - `typed` Simulates typing by outputting characters one at a time.  
  - `instant` Outputs the text using a much faster typing simulation. 
  **Note:** When set to "instant," the application outputs a message in the console: "Future functionality"

## Open API Key

To use this application, you need a valid OpenAI account to create an API key. Additionally, you must have credit on your OpenAI account. The cost of usage is very inexpensive ($5 USD will last you weeks), you need to ensure your account has sufficient credit for the API requests.

### Getting Your OpenAI API Key

1. Visit the [OpenAI API page](https://platform.openai.com/signup/) to create an account if you don’t already have one.
2. Once your account is created, log in to the [OpenAI Dashboard](https://platform.openai.com/account/api-keys).
3. Navigate to the API Keys section and generate a new API key.
4. Copy the generated API key.
5. Open the `config.ini` file (you can use the "Edit Config" option in the system tray menu) and replace the placeholder `your_api_key_here` with your new API key.

### Adding Credits to Your OpenAI Account

1. In your OpenAI Dashboard, navigate to the Billing section.
2. Add a payment method or pre-purchase credits to ensure your API requests are processed.
3. Check your account usage and remaining credits regularly to avoid interruptions.

## Run at Startup

If you would like the application to automatically start when you log in:

1. Right-click on `main.py` and select **Create Shortcut**.
2. Press `Win + R` to open the Run dialog.
3. Type `shell:startup` and press Enter to open the Startup folder.
4. Move the shortcut you created into the Startup folder.
5. The application will now launch automatically when you log in.

## Notes

- Make sure your API key is valid and has sufficient credit.
- This application works best in environments with a stable microphone setup.
- Often best used for inputting prompts via voice into ChatGPT for quick iteration and refinement.


## File Structure

```plaintext
project_root/
├── main.py
├── version.txt
├── assets/
│   ├── default_config.ini
│   ├── icon.ico
│   ├── icon256x256.png
│   └── icon1024x1024.png
├── bin/
│   ├── whisper-dictation.exe
```

## Build Command (Console Present)

```plaintext
pyinstaller --onefile --icon=assets/icon.ico --add-data "version.txt;." --add-data "assets/icon.ico;assets" --add-data "assets/default_config.ini;assets" main.py
```

## Build Command (No Console)

```plaintext
pyinstaller --onefile --windowed --icon=assets/icon.ico --add-data "version.txt;." --add-data "assets/icon.ico;assets" --add-data "assets/default_config.ini;assets" main.py
```


## To Do

- Add ability to see version in the taskbar icon.
- Add/check default config for file reference in executable, likely bug.
- Add logging.
- Make tool adaptable so that any voice to text API/model may be used.
- Add notifications so that you can see the application is running when you execute it and warning messages about the current version.
- Fix ready to record popup appearing every time, only needs to appear once.


## Credits

Originally forked from:
https://github.com/Snuffypot11/Whisper-Dictation


