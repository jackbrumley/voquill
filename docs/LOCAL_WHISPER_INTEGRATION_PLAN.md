# Local Whisper Integration Plan

## Overview

This document outlines the plan to integrate local Whisper model support into Voquill as an alternative to the OpenAI API. This will provide users with offline transcription capabilities, improved privacy, and cost savings.

## Current Architecture Analysis

### Existing Implementation
- **Transcription**: OpenAI's Whisper API via HTTP requests in `transcription.rs`
- **Configuration**: Simple config structure with `openai_api_key` field
- **Frontend**: Tauri-based desktop app with Preact UI
- **Audio**: Recording via `cpal` and `hound` libraries
- **Dependencies**: `reqwest` for HTTP, `serde` for serialization

### Current Limitations
- Requires internet connection
- Ongoing API costs
- Data privacy concerns (audio sent to OpenAI)
- Dependency on external service availability

## Proposed Solution

### High-Level Architecture
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Audio Input   │───▶│ Transcription    │───▶│   Text Output   │
│   (unchanged)   │    │   Service        │    │   (unchanged)   │
└─────────────────┘    │   (abstracted)   │    └─────────────────┘
                       └──────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
            ┌───────▼────────┐  ┌───────▼────────┐
            │ OpenAI Service │  │ Local Whisper  │
            │ (existing)     │  │ Service (new)  │
            └────────────────┘  └────────────────┘
```

## Implementation Plan

### Phase 1: Core Infrastructure

#### 1.1 Configuration Changes
**File**: `src/config.rs`

Extend the config structure:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Transcription method selection
    pub transcription_mode: TranscriptionMode,
    
    // API-based settings (OpenAI, OpenRouter, etc.)
    pub api_key: String,
    pub api_endpoint: String,
    pub api_model: String, // "whisper-1" for OpenAI, model names for other providers
    
    // Local Whisper settings
    pub local_whisper_model_path: Option<String>,
    pub local_whisper_model_size: String, // "tiny", "base", "small", "medium", "large"
    pub local_whisper_device: String,     // "cpu", "gpu" (future)
    
    // Existing fields
    pub hotkey: String,
    pub typing_speed_interval: f64,
    pub pixels_from_bottom: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscriptionMode {
    API,   // Renamed from OpenAI to be more generic
    Local,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            transcription_mode: TranscriptionMode::API,
            api_key: "your_api_key_here".to_string(),
            api_endpoint: "https://api.openai.com/v1/audio/transcriptions".to_string(),
            api_model: "whisper-1".to_string(),
            local_whisper_model_path: None,
            local_whisper_model_size: "base".to_string(),
            local_whisper_device: "cpu".to_string(),
            hotkey: "ctrl+space".to_string(),
            typing_speed_interval: 0.01,
            pixels_from_bottom: 50,
        }
    }
}
```

#### 1.2 Dependencies
**File**: `Cargo.toml`

Add new dependencies:
```toml
[dependencies]
# Existing dependencies...

# Local Whisper support
whisper-rs = "0.10"  # Rust bindings for whisper.cpp
# Alternative: candle-whisper = "0.3"  # Pure Rust implementation

# Model downloading
tokio-util = { version = "0.7", features = ["io"] }
futures-util = "0.3"
sha2 = "0.10"  # For model integrity verification
```

#### 1.3 Transcription Service Abstraction
**File**: `src/transcription.rs`

Create trait-based architecture:
```rust
use async_trait::async_trait;

#[derive(Debug)]
pub enum TranscriptionError {
    NetworkError(String),
    ModelError(String),
    AudioError(String),
    ConfigError(String),
}

#[async_trait]
pub trait TranscriptionService {
    async fn transcribe(&self, audio_data: &[u8]) -> Result<String, TranscriptionError>;
    async fn test_connection(&self) -> Result<bool, TranscriptionError>;
    fn service_name(&self) -> &'static str;
}

pub struct TranscriptionManager {
    current_service: Box<dyn TranscriptionService + Send + Sync>,
}

impl TranscriptionManager {
    pub fn new(config: &Config) -> Result<Self, TranscriptionError> {
        let service: Box<dyn TranscriptionService + Send + Sync> = match config.transcription_mode {
            TranscriptionMode::API => {
                Box::new(APITranscriptionService::new(
                    &config.api_key,
                    &config.api_endpoint,
                    &config.api_model
                )?)
            }
            TranscriptionMode::Local => {
                Box::new(LocalWhisperService::new(config)?)
            }
        };
        
        Ok(Self {
            current_service: service,
        })
    }
    
    pub async fn transcribe(&self, audio_data: &[u8]) -> Result<String, TranscriptionError> {
        self.current_service.transcribe(audio_data).await
    }
}

// Updated API service to support configurable endpoints
pub struct APITranscriptionService {
    client: reqwest::Client,
    api_key: String,
    api_endpoint: String,
    api_model: String,
}

impl APITranscriptionService {
    pub fn new(api_key: &str, api_endpoint: &str, api_model: &str) -> Result<Self, TranscriptionError> {
        Ok(Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            api_endpoint: api_endpoint.to_string(),
            api_model: api_model.to_string(),
        })
    }
}

#[async_trait]
impl TranscriptionService for APITranscriptionService {
    async fn transcribe(&self, audio_data: &[u8]) -> Result<String, TranscriptionError> {
        let form = multipart::Form::new()
            .part(
                "file",
                multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| TranscriptionError::AudioError(e.to_string()))?,
            )
            .text("model", &self.api_model);

        let response = self.client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;
            return Err(TranscriptionError::NetworkError(format!("API error: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;
        
        let text = json["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
    
    async fn test_connection(&self) -> Result<bool, TranscriptionError> {
        // Create a minimal test audio file (silence)
        let test_audio = create_test_audio();
        
        let form = multipart::Form::new()
            .part(
                "file",
                multipart::Part::bytes(test_audio)
                    .file_name("test.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| TranscriptionError::AudioError(e.to_string()))?,
            )
            .text("model", &self.api_model);

        let response = self.client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

        Ok(response.status().is_success())
    }
    
    fn service_name(&self) -> &'static str {
        "API Transcription"
    }
}
```

### Phase 2: Model Management System

#### 2.1 Model Manager
**File**: `src/model_manager.rs`

```rust
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct ModelManager {
    models_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub size: String,
    pub file_size: u64,
    pub download_url: String,
    pub sha256: String,
    pub description: String,
}

impl ModelManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let models_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("voquill")
            .join("models");
        
        std::fs::create_dir_all(&models_dir)?;
        
        Ok(Self { models_dir })
    }
    
    pub fn get_available_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                size: "tiny".to_string(),
                file_size: 39_000_000,  // ~39MB
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".to_string(),
                sha256: "...".to_string(),
                description: "Fastest, least accurate. Good for testing.".to_string(),
            },
            ModelInfo {
                size: "base".to_string(),
                file_size: 142_000_000, // ~142MB
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".to_string(),
                sha256: "...".to_string(),
                description: "Good balance of speed and accuracy.".to_string(),
            },
            ModelInfo {
                size: "small".to_string(),
                file_size: 466_000_000, // ~466MB
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".to_string(),
                sha256: "...".to_string(),
                description: "Better accuracy, slower processing.".to_string(),
            },
            // Add medium and large models...
        ]
    }
    
    pub async fn download_model(
        &self,
        model_info: &ModelInfo,
        progress_callback: impl Fn(u64, u64) + Send + 'static,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Implementation for downloading with progress tracking
        todo!()
    }
    
    pub fn is_model_downloaded(&self, model_size: &str) -> bool {
        self.get_model_path(model_size).exists()
    }
    
    pub fn get_model_path(&self, model_size: &str) -> PathBuf {
        self.models_dir.join(format!("ggml-{}.bin", model_size))
    }
}
```

#### 2.2 Local Whisper Service
**File**: `src/local_whisper.rs`

```rust
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

pub struct LocalWhisperService {
    model_path: PathBuf,
    context: Option<WhisperContext>,
}

impl LocalWhisperService {
    pub fn new(config: &Config) -> Result<Self, TranscriptionError> {
        let model_manager = ModelManager::new()
            .map_err(|e| TranscriptionError::ConfigError(e.to_string()))?;
        
        let model_path = model_manager.get_model_path(&config.local_whisper_model_size);
        
        if !model_path.exists() {
            return Err(TranscriptionError::ModelError(
                format!("Model {} not found. Please download it first.", config.local_whisper_model_size)
            ));
        }
        
        Ok(Self {
            model_path,
            context: None,
        })
    }
    
    fn ensure_context_loaded(&mut self) -> Result<&WhisperContext, TranscriptionError> {
        if self.context.is_none() {
            let ctx = WhisperContext::new_with_params(
                self.model_path.to_str().unwrap(),
                WhisperContextParameters::default(),
            ).map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
            
            self.context = Some(ctx);
        }
        
        Ok(self.context.as_ref().unwrap())
    }
}

#[async_trait]
impl TranscriptionService for LocalWhisperService {
    async fn transcribe(&mut self, audio_data: &[u8]) -> Result<String, TranscriptionError> {
        // Convert audio_data to the format expected by whisper-rs
        // This will require audio format conversion
        let audio_samples = convert_audio_to_samples(audio_data)?;
        
        let ctx = self.ensure_context_loaded()?;
        
        // Configure transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        
        // Run transcription
        ctx.full(params, &audio_samples)
            .map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
        
        // Extract text
        let num_segments = ctx.full_n_segments()
            .map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
        
        let mut result = String::new();
        for i in 0..num_segments {
            let segment_text = ctx.full_get_segment_text(i)
                .map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
            result.push_str(&segment_text);
        }
        
        Ok(result.trim().to_string())
    }
    
    async fn test_connection(&self) -> Result<bool, TranscriptionError> {
        // For local models, just check if the model file exists and is readable
        Ok(self.model_path.exists())
    }
    
    fn service_name(&self) -> &'static str {
        "Local Whisper"
    }
}
```

### Phase 3: UI Integration

#### 3.1 Frontend Configuration Updates
**File**: `ui/src/App.tsx`

Add to the Config interface:
```typescript
interface Config {
  transcription_mode: 'API' | 'Local';
  api_key: string;
  api_endpoint: string;
  api_model: string;
  local_whisper_model_path?: string;
  local_whisper_model_size: string;
  local_whisper_device: string;
  hotkey: string;
  typing_speed_interval: number;
  pixels_from_bottom: number;
}

interface ModelInfo {
  size: string;
  file_size: number;
  description: string;
  is_downloaded: boolean;
}
```

#### 3.2 Model Management UI Components
Add to the Config tab:
```tsx
// Transcription Mode Selection
<div className="form-group">
  <label>Transcription Method:</label>
  <div className="radio-group">
    <label>
      <input
        type="radio"
        name="transcription_mode"
        value="OpenAI"
        checked={config.transcription_mode === 'OpenAI'}
        onChange={(e) => updateConfig('transcription_mode', e.target.value)}
      />
      OpenAI API
    </label>
    <label>
      <input
        type="radio"
        name="transcription_mode"
        value="Local"
        checked={config.transcription_mode === 'Local'}
        onChange={(e) => updateConfig('transcription_mode', e.target.value)}
      />
      Local Whisper
    </label>
  </div>
</div>

// Model Management Section (shown when Local is selected)
{config.transcription_mode === 'Local' && (
  <div className="model-management">
    <h3>Whisper Model Management</h3>
    
    <div className="form-group">
      <label>Model Size:</label>
      <select
        value={config.local_whisper_model_size}
        onChange={(e) => updateConfig('local_whisper_model_size', e.target.value)}
      >
        {availableModels.map(model => (
          <option key={model.size} value={model.size}>
            {model.size} - {formatFileSize(model.file_size)} - {model.description}
          </option>
        ))}
      </select>
    </div>
    
    <div className="model-status">
      {getModelStatus(config.local_whisper_model_size)}
    </div>
    
    <div className="form-actions">
      <button 
        className="button primary"
        onClick={() => downloadModel(config.local_whisper_model_size)}
        disabled={isDownloading}
      >
        {isDownloading ? 'Downloading...' : 'Download Model'}
      </button>
    </div>
    
    {downloadProgress > 0 && (
      <div className="progress-bar">
        <div 
          className="progress-fill" 
          style={{ width: `${downloadProgress}%` }}
        />
        <span className="progress-text">{downloadProgress.toFixed(1)}%</span>
      </div>
    )}
  </div>
)}
```

#### 3.3 Backend Commands
**File**: `src/main.rs`

Add new Tauri commands:
```rust
#[tauri::command]
async fn get_available_models() -> Result<Vec<ModelInfo>, String> {
    Ok(ModelManager::get_available_models())
}

#[tauri::command]
async fn download_model(model_size: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let model_manager = ModelManager::new()
        .map_err(|e| e.to_string())?;
    
    let models = ModelManager::get_available_models();
    let model_info = models.iter()
        .find(|m| m.size == model_size)
        .ok_or("Model not found")?;
    
    // Download with progress updates
    model_manager.download_model(model_info, |downloaded, total| {
        let progress = (downloaded as f64 / total as f64) * 100.0;
        app_handle.emit_all("download-progress", progress).ok();
    }).await
    .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn check_model_status(model_size: String) -> Result<bool, String> {
    let model_manager = ModelManager::new()
        .map_err(|e| e.to_string())?;
    
    Ok(model_manager.is_model_downloaded(&model_size))
}
```

### Phase 4: Integration and Testing

#### 4.1 Main Application Updates
**File**: `src/main.rs`

Update the main transcription flow:
```rust
// Replace direct OpenAI calls with TranscriptionManager
let config = load_config()?;
let transcription_manager = TranscriptionManager::new(&config)?;

// In the recording handler
let transcription_result = transcription_manager
    .transcribe(&audio_data)
    .await?;
```

#### 4.2 Error Handling and Fallbacks
- Graceful degradation when models are missing
- Clear error messages for common issues
- Automatic fallback suggestions (e.g., "Model not found, would you like to download it?")

#### 4.3 Performance Optimizations
- Lazy model loading (load on first use)
- Model caching in memory
- Background model preloading option
- CPU vs GPU detection and optimization

### Phase 5: Documentation and Polish

#### 5.1 User Documentation
- Model size comparison table
- Performance expectations
- Storage requirements
- Troubleshooting guide

#### 5.2 Developer Documentation
- Architecture overview
- Adding new transcription services
- Model format specifications
- Testing procedures

## API Provider Support

### Supported Whisper API Providers

The configurable API endpoint approach allows users to choose from multiple Whisper API providers:

#### OpenAI (Default)
- **Endpoint**: `https://api.openai.com/v1/audio/transcriptions`
- **Model**: `whisper-1`
- **Authentication**: Bearer token
- **Pricing**: Pay-per-use
- **Features**: High accuracy, reliable service

#### OpenRouter
- **Endpoint**: `https://openrouter.ai/api/v1/audio/transcriptions`
- **Model**: `whisper-1` (or provider-specific models)
- **Authentication**: Bearer token
- **Pricing**: Competitive rates, multiple model options
- **Features**: Access to multiple providers through one API

#### Groq
- **Endpoint**: `https://api.groq.com/openai/v1/audio/transcriptions`
- **Model**: `whisper-large-v3`
- **Authentication**: Bearer token
- **Pricing**: Fast inference, competitive pricing
- **Features**: Ultra-fast processing

#### Together AI
- **Endpoint**: `https://api.together.xyz/v1/audio/transcriptions`
- **Model**: `whisperx`
- **Authentication**: Bearer token
- **Pricing**: Cost-effective
- **Features**: Good balance of speed and cost

### API Configuration Presets

To make it easier for users, we can provide preset configurations:

```rust
pub struct APIPreset {
    pub name: String,
    pub endpoint: String,
    pub default_model: String,
    pub description: String,
}

impl APIPreset {
    pub fn get_presets() -> Vec<APIPreset> {
        vec![
            APIPreset {
                name: "OpenAI".to_string(),
                endpoint: "https://api.openai.com/v1/audio/transcriptions".to_string(),
                default_model: "whisper-1".to_string(),
                description: "Official OpenAI Whisper API - highest reliability".to_string(),
            },
            APIPreset {
                name: "OpenRouter".to_string(),
                endpoint: "https://openrouter.ai/api/v1/audio/transcriptions".to_string(),
                default_model: "whisper-1".to_string(),
                description: "Access multiple providers through OpenRouter".to_string(),
            },
            APIPreset {
                name: "Groq".to_string(),
                endpoint: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
                default_model: "whisper-large-v3".to_string(),
                description: "Ultra-fast inference with Groq's hardware".to_string(),
            },
            APIPreset {
                name: "Custom".to_string(),
                endpoint: "".to_string(),
                default_model: "whisper-1".to_string(),
                description: "Configure your own API endpoint".to_string(),
            },
        ]
    }
}
```

### Enhanced UI for API Selection

```tsx
// API Provider Selection (shown when API mode is selected)
{config.transcription_mode === 'API' && (
  <div className="api-configuration">
    <div className="form-group">
      <label>API Provider:</label>
      <select
        value={selectedPreset}
        onChange={(e) => handlePresetChange(e.target.value)}
      >
        {apiPresets.map(preset => (
          <option key={preset.name} value={preset.name}>
            {preset.name} - {preset.description}
          </option>
        ))}
      </select>
    </div>
    
    <div className="form-group">
      <label htmlFor="api-key">API Key:</label>
      <input
        type="password"
        id="api-key"
        placeholder="Enter your API key"
        value={config.api_key}
        onChange={(e) => updateConfig('api_key', e.target.value)}
      />
    </div>
    
    <div className="form-group">
      <label htmlFor="api-endpoint">API Endpoint:</label>
      <input
        type="url"
        id="api-endpoint"
        placeholder="https://api.example.com/v1/audio/transcriptions"
        value={config.api_endpoint}
        onChange={(e) => updateConfig('api_endpoint', e.target.value)}
        disabled={selectedPreset !== 'Custom'}
      />
    </div>
    
    <div className="form-group">
      <label htmlFor="api-model">Model:</label>
      <input
        type="text"
        id="api-model"
        placeholder="whisper-1"
        value={config.api_model}
        onChange={(e) => updateConfig('api_model', e.target.value)}
      />
    </div>
  </div>
)}
```

## Technical Considerations

### Model Sizes and Performance
| Model | Size | Speed | Accuracy | Use Case |
|-------|------|-------|----------|----------|
| tiny  | 39MB | Fastest | Basic | Testing, low-resource devices |
| base  | 142MB | Fast | Good | General use, recommended default |
| small | 466MB | Medium | Better | Higher accuracy needs |
| medium| 769MB | Slow | High | Professional use |
| large | 1.5GB | Slowest | Highest | Maximum accuracy |

### API Provider Comparison
| Provider | Speed | Cost | Reliability | Special Features |
|----------|-------|------|-------------|------------------|
| OpenAI | Medium | High | Excellent | Official, most reliable |
| OpenRouter | Medium | Medium | Good | Multiple providers, flexibility |
| Groq | Very Fast | Medium | Good | Hardware acceleration |
| Together AI | Fast | Low | Good | Cost-effective |

### System Requirements
- **Minimum RAM**: 1GB free (for tiny model)
- **Recommended RAM**: 4GB free (for base model)
- **Storage**: 150MB - 2GB depending on model
- **CPU**: Modern multi-core processor recommended

### Privacy and Security
- All processing happens locally
- No data sent to external services
- Models stored in user's config directory
- Optional model encryption for sensitive environments

## Migration Strategy

### Backward Compatibility
- Existing OpenAI API functionality remains unchanged
- Default configuration uses OpenAI mode
- Seamless switching between modes
- No breaking changes to existing workflows

### Gradual Rollout
1. **Phase 1**: Core infrastructure (no UI changes)
2. **Phase 2**: Basic local transcription (expert users)
3. **Phase 3**: Full UI integration (general users)
4. **Phase 4**: Advanced features and optimizations

## Future Enhancements

### Potential Improvements
- **GPU acceleration**: CUDA/Metal support for faster inference
- **Custom models**: Support for fine-tuned models
- **Language selection**: Multi-language support
- **Real-time transcription**: Streaming audio processing
- **Model updates**: Automatic model version management
- **Quantized models**: Smaller, faster model variants

### Integration Opportunities
- **Voice commands**: Beyond transcription to command recognition
- **Speaker identification**: Multi-speaker scenarios
- **Noise reduction**: Audio preprocessing improvements
- **Cloud sync**: Optional model sharing across devices

## Success Metrics

### Technical Metrics
- Model download success rate > 95%
- Local transcription accuracy within 5% of OpenAI
- Startup time increase < 2 seconds
- Memory usage < 500MB for base model

### User Experience Metrics
- Setup completion rate > 80%
- User preference for local vs API
- Support ticket reduction for privacy concerns
- Performance satisfaction ratings

## Conclusion

This plan provides a comprehensive roadmap for integrating local Whisper support into Voquill while maintaining the existing OpenAI API functionality. The modular approach ensures we can implement features incrementally and provides users with flexible transcription options based on their needs for privacy, cost, and performance.

The implementation prioritizes user experience with clear setup flows, progress indicators, and helpful error messages, while maintaining the technical robustness expected from a professional transcription tool.
