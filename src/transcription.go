package main

import (
	"bytes"
	"encoding/json"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
)

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
