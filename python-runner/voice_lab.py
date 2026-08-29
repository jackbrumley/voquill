"""
Voquill Voice Lab — Interactive Acoustic Preset Designer & Tuner.
Provides a local web GUI to test base voices, dial in custom DSP parameters,
preview audio with opening/closing chimes in stereo, and save voice presets to JSON.
"""

from __future__ import annotations

import io
import json
import logging
import os
import sys
import tempfile
import urllib.request
import tarfile
from typing import Any, Dict, Optional

import fastapi
from fastapi import FastAPI, HTTPException
from fastapi.responses import HTMLResponse, Response
from fastapi.middleware.cors import CORSMiddleware
import numpy as np
import pydantic
from pydantic import BaseModel
from scipy import signal
import soundfile as sf
import uvicorn

# Include local tts module
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPT_DIR)
from tts import dsp

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("voquill.voice_lab")

app = FastAPI(title="Voquill Voice Lab", version="1.0.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

PRESETS_FILE = os.path.join(SCRIPT_DIR, "voice_presets.json")
SAMPLES_LOG_FILE = os.path.join(SCRIPT_DIR, "samples_log.json")
MODELS_DIR = os.path.join(os.path.expanduser("~/.config/voquill-app/models/tts"))
os.makedirs(MODELS_DIR, exist_ok=True)

# Catalog of available base models
AVAILABLE_MODELS = {
    "piper-en_GB-northern_english_male-medium": {
        "label": "🇬🇧 Northern English Male (SAS Price / Tactical)",
        "archive": "vits-piper-en_GB-northern_english_male-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-northern_english_male-medium.tar.bz2",
        "model_file": "en_GB-northern_english_male-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_GB-alan-medium": {
        "label": "🇬🇧 Alan (Cold British Commander / Dark Baritone)",
        "archive": "vits-piper-en_GB-alan-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-alan-medium.tar.bz2",
        "model_file": "en_GB-alan-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-norman-medium": {
        "label": "🇺🇸 Norman (Deep American Baritone / Dispatcher)",
        "archive": "vits-piper-en_US-norman-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-norman-medium.tar.bz2",
        "model_file": "en_US-norman-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-joe-medium": {
        "label": "🇺🇸 Joe (Gritty Older Combat Veteran)",
        "archive": "vits-piper-en_US-joe-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-joe-medium.tar.bz2",
        "model_file": "en_US-joe-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-bryce-medium": {
        "label": "🇺🇸 Bryce (High-Energy / Commanding Operator)",
        "archive": "vits-piper-en_US-bryce-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-bryce-medium.tar.bz2",
        "model_file": "en_US-bryce-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-danny-low": {
        "label": "🇺🇸 Danny (Fast Tactical Field Operator)",
        "archive": "vits-piper-en_US-danny-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-danny-low.tar.bz2",
        "model_file": "en_US-danny-low.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-ryan-low": {
        "label": "🇺🇸 Ryan (Deep Male / Titan Base)",
        "archive": "vits-piper-en_US-ryan-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-ryan-low.tar.bz2",
        "model_file": "en_US-ryan-low.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-amy-low": {
        "label": "🛸 Amy (Cyberpunk EVA / Clear Sci-Fi Female)",
        "archive": "vits-piper-en_US-amy-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-low.tar.bz2",
        "model_file": "en_US-amy-low.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_GB-cori-medium": {
        "label": "🇬🇧 Cori (Expressive British Female)",
        "archive": "vits-piper-en_GB-cori-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-cori-medium.tar.bz2",
        "model_file": "en_GB-cori-medium.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-glados": {
        "label": "🤖 GLaDOS (Iconic Robotic Portal AI)",
        "archive": "vits-piper-en_US-glados",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-glados.tar.bz2",
        "model_file": "en_US-glados.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_GB-southern_english_female-low": {
        "label": "✈️ Southern English Female (Flight Deck ATC)",
        "archive": "vits-piper-en_GB-southern_english_female-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-southern_english_female-low.tar.bz2",
        "model_file": "en_GB-southern_english_female-low.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-libritts_r-medium": {
        "label": "🎭 LibriTTS-R Multi-Speaker (904 Speakers, e.g. 700 Titan, 200 Dispatcher)",
        "archive": "vits-piper-en_US-libritts_r-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-libritts_r-medium.tar.bz2",
        "model_file": "en_US-libritts_r-medium.onnx",
        "is_multi_speaker": True,
    },
}

_TTS_INSTANCES = {}


def ensure_model(model_key: str) -> Dict[str, str]:
    meta = AVAILABLE_MODELS.get(model_key)
    if not meta:
        raise ValueError(f"Unknown model '{model_key}'")

    vdir = os.path.join(MODELS_DIR, meta["archive"])
    mpath = os.path.join(vdir, meta["model_file"])
    tpath = os.path.join(vdir, "tokens.txt")
    dpath = os.path.join(vdir, "espeak-ng-data")

    if os.path.exists(mpath) and os.path.exists(tpath) and os.path.exists(dpath):
        return {"model": mpath, "tokens": tpath, "data": dpath}

    arch_path = os.path.join(MODELS_DIR, f"{meta['archive']}.tar.bz2")
    logger.info("Downloading TTS model %s...", model_key)
    urllib.request.urlretrieve(meta["url"], arch_path)

    logger.info("Extracting %s...", model_key)
    with tarfile.open(arch_path, "r:bz2") as tar:
        tar.extractall(path=MODELS_DIR)

    if os.path.exists(arch_path):
        os.remove(arch_path)

    return {"model": mpath, "tokens": tpath, "data": dpath}


def get_tts_engine(model_key: str, noise_scale=0.667, noise_scale_w=0.8, length_scale=1.0):
    cache_key = f"{model_key}_{noise_scale}_{length_scale}"
    if cache_key in _TTS_INSTANCES:
        return _TTS_INSTANCES[cache_key]

    import sherpa_onnx

    paths = ensure_model(model_key)
    tts = sherpa_onnx.OfflineTts(
        sherpa_onnx.OfflineTtsConfig(
            model=sherpa_onnx.OfflineTtsModelConfig(
                vits=sherpa_onnx.OfflineTtsVitsModelConfig(
                    model=paths["model"],
                    tokens=paths["tokens"],
                    data_dir=paths["data"],
                    noise_scale=noise_scale,
                    noise_scale_w=noise_scale_w,
                    length_scale=length_scale,
                ),
                num_threads=2,
            )
        )
    )
    _TTS_INSTANCES[cache_key] = tts
    return tts


class SynthesizeLabRequest(BaseModel):
    text: str
    model_key: str
    speaker_id: int = 0
    speed: float = 1.0
    noise_scale: float = 0.667
    pitch: float = 0.0
    sub_bass: float = 0.0
    comb_mix: float = 0.0
    flanger_mix: float = 0.0
    radio_bandpass: bool = False
    radio_drive: float = 1.0
    rf_noise: float = 0.0
    opening_chime: str = "none"
    closing_chime: str = "none"


class VoicePreset(BaseModel):
    id: str
    name: str
    category: str
    description: str
    model_key: str
    speaker_id: int = 0
    speed: float = 1.0
    pitch: float = 0.0
    sub_bass: float = 0.0
    comb_mix: float = 0.0
    flanger_mix: float = 0.0
    radio_bandpass: bool = False
    radio_drive: float = 1.0
    rf_noise: float = 0.0
    opening_chime: str = "none"
    closing_chime: str = "none"


def load_saved_presets() -> list[dict]:
    if os.path.exists(PRESETS_FILE):
        try:
            with open(PRESETS_FILE, "r") as f:
                return json.load(f)
        except Exception:
            return []
    return []


def save_presets(presets: list[dict]):
    with open(PRESETS_FILE, "w") as f:
        json.dump(presets, f, indent=2)


@app.get("/api/models")
def get_models():
    return [
        {
            "id": k,
            "label": v["label"],
            "is_multi_speaker": v.get("is_multi_speaker", False),
        }
        for k, v in AVAILABLE_MODELS.items()
    ]


@app.get("/api/presets")
def get_presets():
    return load_saved_presets()


@app.post("/api/presets")
def save_preset(preset: VoicePreset):
    presets = load_saved_presets()
    # Replace existing or append
    presets = [p for p in presets if p["id"] != preset.id]
    presets.append(preset.model_dump())
    save_presets(presets)
    return {"status": "ok", "count": len(presets)}


@app.delete("/api/presets/{preset_id}")
def delete_preset(preset_id: str):
    presets = load_saved_presets()
    presets = [p for p in presets if p["id"] != preset_id]
    save_presets(presets)
    return {"status": "ok", "count": len(presets)}


@app.get("/api/samples_log")
def get_samples_log():
    if os.path.exists(SAMPLES_LOG_FILE):
        try:
            with open(SAMPLES_LOG_FILE, "r") as f:
                return json.load(f)
        except Exception:
            pass
    return {"samples": []}


@app.post("/api/synthesize")
def synthesize_audio(req: SynthesizeLabRequest):
    try:
        tts = get_tts_engine(req.model_key, noise_scale=req.noise_scale, length_scale=1.0)
        audio = tts.generate(req.text.strip(), sid=req.speaker_id, speed=req.speed)

        if len(audio.samples) == 0:
            raise HTTPException(status_code=500, detail="TTS generated 0 samples")

        raw = np.array(audio.samples, dtype=np.float32)

        # Apply interactive DSP pipeline
        processed = dsp.apply_custom_dsp(
            samples=raw,
            sample_rate=audio.sample_rate,
            pitch=req.pitch,
            sub_bass=req.sub_bass,
            comb_mix=req.comb_mix,
            flanger_mix=req.flanger_mix,
            radio_bandpass=req.radio_bandpass,
            radio_drive=req.radio_drive,
            rf_noise=req.rf_noise,
            opening_chime=req.opening_chime,
            closing_chime=req.closing_chime,
        )

        # Convert to 2-channel stereo so headphones hear in both ears
        stereo = dsp.to_stereo(processed)

        out_io = io.BytesIO()
        sf.write(out_io, stereo, audio.sample_rate, format="WAV")
        out_io.seek(0)

        return Response(content=out_io.read(), media_type="audio/wav")

    except Exception as e:
        logger.exception("Synthesis error")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/", response_class=HTMLResponse)
def index_page():
    return HTML_CONTENT


HTML_CONTENT = """<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Voquill Voice Lab — Interactive Acoustic Preset Designer</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Plus+Jakarta+Sans:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
    :root {
      --bg: #090d16;
      --card-bg: #111726;
      --card-border: rgba(255, 255, 255, 0.08);
      --primary: #6366f1;
      --primary-hover: #4f46e5;
      --accent: #38bdf8;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --success: #34d399;
      --danger: #f87171;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Plus Jakarta Sans', sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.5;
      padding: 24px;
      min-height: 100vh;
    }
    .container {
      max-width: 1200px;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 20px;
    }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      border-bottom: 1px solid var(--card-border);
      padding-bottom: 16px;
    }
    h1 {
      font-size: 22px;
      font-weight: 700;
      display: flex;
      align-items: center;
      gap: 10px;
      color: #fff;
    }
    .badge {
      background: rgba(99, 102, 241, 0.2);
      border: 1px solid rgba(99, 102, 241, 0.4);
      color: #c7d2fe;
      font-size: 11px;
      font-weight: 600;
      padding: 2px 8px;
      border-radius: 9999px;
      font-family: 'JetBrains Mono', monospace;
    }
    .tabs {
      display: flex;
      gap: 8px;
    }
    .tab-btn {
      background: rgba(255, 255, 255, 0.04);
      border: 1px solid var(--card-border);
      color: var(--text-muted);
      padding: 8px 16px;
      border-radius: 6px;
      cursor: pointer;
      font-size: 13px;
      font-weight: 600;
      transition: all 0.15s;
    }
    .tab-btn.active {
      background: var(--primary);
      color: #fff;
      border-color: var(--primary);
    }
    .grid-2 {
      display: grid;
      grid-template-columns: 1.2fr 0.8fr;
      gap: 20px;
    }
    .card {
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-radius: 10px;
      padding: 18px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .card-title {
      font-size: 13px;
      font-weight: 700;
      color: var(--accent);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      display: flex;
      align-items: center;
      gap: 6px;
    }
    label {
      font-size: 12px;
      font-weight: 600;
      color: var(--text-muted);
      display: flex;
      justify-content: space-between;
    }
    input[type="text"], select, textarea {
      width: 100%;
      background: rgba(0, 0, 0, 0.35);
      border: 1px solid var(--card-border);
      border-radius: 6px;
      color: #fff;
      padding: 9px 12px;
      font-size: 13px;
      outline: none;
      font-family: inherit;
    }
    input[type="text"]:focus, select:focus, textarea:focus {
      border-color: var(--primary);
    }
    .quick-phrases {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    .chip-btn {
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid var(--card-border);
      color: var(--text-muted);
      font-size: 11px;
      padding: 4px 8px;
      border-radius: 4px;
      cursor: pointer;
    }
    .chip-btn:hover {
      background: rgba(99, 102, 241, 0.2);
      color: #c7d2fe;
      border-color: rgba(99, 102, 241, 0.4);
    }
    .slider-row {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .slider-container {
      display: flex;
      align-items: center;
      gap: 12px;
    }
    input[type="range"] {
      flex: 1;
      accent-color: var(--primary);
      cursor: pointer;
    }
    .slider-val {
      font-family: 'JetBrains Mono', monospace;
      font-size: 12px;
      width: 50px;
      text-align: right;
      color: #cbd5e1;
    }
    .btn {
      padding: 10px 18px;
      border-radius: 6px;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      border: none;
      transition: all 0.15s;
    }
    .btn-primary {
      background: var(--primary);
      color: #fff;
    }
    .btn-primary:hover {
      background: var(--primary-hover);
    }
    .btn-success {
      background: rgba(52, 211, 153, 0.2);
      color: #34d399;
      border: 1px solid rgba(52, 211, 153, 0.4);
    }
    .btn-success:hover {
      background: rgba(52, 211, 153, 0.3);
    }
    audio {
      width: 100%;
      height: 42px;
      border-radius: 6px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 12px;
    }
    th, td {
      text-align: left;
      padding: 8px 10px;
      border-bottom: 1px solid var(--card-border);
    }
    th { color: var(--text-muted); font-weight: 600; }
    .status-keep { color: #34d399; font-weight: 600; }
    .status-relabel { color: #38bdf8; font-weight: 600; }
    .status-discard { color: #f87171; }
  </style>
</head>
<body>

<div class="container">
  <header>
    <h1>🎙️ Voquill Voice Lab <span class="badge">Acoustic Preset Designer</span></h1>
    <div class="tabs">
      <button class="tab-btn active" onclick="switchTab('tuner')">Interactive Tuner</button>
      <button class="tab-btn" onclick="switchTab('presets')">Saved Presets</button>
      <button class="tab-btn" onclick="switchTab('feedback')">Sample History (1–27)</button>
    </div>
  </header>

  <!-- TAB 1: TUNER -->
  <div id="tab-tuner" class="grid-2">
    <!-- Left Column: Source Voice & Text -->
    <div class="card">
      <div class="card-title">1. Base Voice & Spoken Phrase</div>
      
      <div style="display:flex; flex-direction:column; gap:6px;">
        <label>Base Neural Model</label>
        <select id="model_key" onchange="onModelChange()">
          <!-- Populated by JS -->
        </select>
      </div>

      <div id="speaker_box" style="display:none; flex-direction:column; gap:6px;">
        <label>Speaker ID (0 to 903) <span id="val_spk" class="slider-val">700</span></label>
        <input type="range" id="speaker_id" min="0" max="903" step="1" value="700" oninput="updateVal('spk', this.value)">
      </div>

      <div style="display:flex; flex-direction:column; gap:6px;">
        <label>Spoken Test Phrase</label>
        <textarea id="test_text" rows="3">Titan online. Core temperature nominal. All weapon systems combat ready.</textarea>
      </div>

      <div style="display:flex; flex-direction:column; gap:6px;">
        <label>Quick Preset Phrases</label>
        <div class="quick-phrases">
          <button class="chip-btn" onclick="setText('Titan online. Core temperature nominal. All weapon systems combat ready.')">⚔️ Titan Mech</button>
          <button class="chip-btn" onclick="setText('Bravo Six, going dark. Target neutralized, requesting immediate extraction.')">📻 Tactical SAS</button>
          <button class="chip-btn" onclick="setText('Target down! Air strike inbound on marked coordinates! Heads down, NOW!')">🔥 Shouting Tactical</button>
          <button class="chip-btn" onclick="setText('Maximum armor engaged. Energy levels at one hundred percent.')">🛡️ Nanosuit</button>
          <button class="chip-btn" onclick="setText('I am vengeance. I am the night. Entering the dark.')">🦇 Dark Knight</button>
          <button class="chip-btn" onclick="setText('All automated workflows completed successfully.')">🎙️ Studio Clean</button>
        </div>
      </div>

      <div class="card-title" style="margin-top:10px;">2. Audio Playback (Both Ears - Stereo)</div>
      <button id="btn_generate" class="btn btn-primary" onclick="generateAudio()" style="padding:12px;">
        <span>▶ Generate & Play Audio</span>
      </button>

      <audio id="audio_player" controls autoplay style="display:none;"></audio>
      <div id="status_msg" style="font-size:11.5px; color:var(--text-muted);">Ready to synthesize.</div>
    </div>

    <!-- Right Column: DSP Knobs & Chimes -->
    <div class="card">
      <div class="card-title" style="display:flex; justify-content:space-between; align-items:center;">
        <span>3. Acoustic DSP Tuning Knobs</span>
        <button class="chip-btn" onclick="resetDspKnobs()" style="text-transform:none; font-weight:600; color:#c7d2fe; background:rgba(99,102,241,0.18); border-color:rgba(99,102,241,0.4); display:flex; align-items:center; gap:4px; padding:3px 8px;">
          <span>↺ Reset DSP to Default</span>
        </button>
      </div>

      <div class="slider-row">
        <label>Pitch Shift <span id="val_pitch" class="slider-val">0.0 st</span></label>
        <div class="slider-container">
          <input type="range" id="pitch" min="-12" max="12" step="0.5" value="0" oninput="updateVal('pitch', this.value + ' st')">
        </div>
      </div>

      <div class="slider-row">
        <label>Playback Speed <span id="val_speed" class="slider-val">1.00x</span></label>
        <div class="slider-container">
          <input type="range" id="speed" min="0.6" max="1.5" step="0.05" value="1.0" oninput="updateVal('speed', this.value + 'x')">
        </div>
      </div>

      <div class="slider-row">
        <label>Vocal Grit / Breathiness (Noise Scale) <span id="val_noise_scale" class="slider-val">0.67</span></label>
        <div class="slider-container">
          <input type="range" id="noise_scale" min="0.5" max="1.0" step="0.05" value="0.667" oninput="updateVal('noise_scale', this.value)">
        </div>
      </div>

      <div class="slider-row">
        <label>Sub-Bass Weight (<140Hz Chest) <span id="val_sub_bass" class="slider-val">0%</span></label>
        <div class="slider-container">
          <input type="range" id="sub_bass" min="0" max="1" step="0.05" value="0" oninput="updateVal('sub_bass', Math.round(this.value*100) + '%')">
        </div>
      </div>

      <div class="slider-row">
        <label>Cockpit Comb Filter (Metallic Hull) <span id="val_comb_mix" class="slider-val">0%</span></label>
        <div class="slider-container">
          <input type="range" id="comb_mix" min="0" max="1" step="0.05" value="0" oninput="updateVal('comb_mix', Math.round(this.value*100) + '%')">
        </div>
      </div>

      <div class="slider-row">
        <label>Robotic Flanger (Synthetic Shimmer) <span id="val_flanger_mix" class="slider-val">0%</span></label>
        <div class="slider-container">
          <input type="range" id="flanger_mix" min="0" max="1" step="0.05" value="0" oninput="updateVal('flanger_mix', Math.round(this.value*100) + '%')">
        </div>
      </div>

      <div style="display:grid; grid-template-columns:1fr 1fr; gap:10px; margin-top:4px;">
        <div style="display:flex; flex-direction:column; gap:4px;">
          <label>Opening Radio Beep / Chime</label>
          <select id="opening_chime">
            <option value="none">None</option>
            <option value="tactical_double_beep">Tactical Double Beep (CS style)</option>
            <option value="radio_click">Subtle Radio Key-Click</option>
            <option value="cockpit_chime">Sci-Fi Cockpit Chime</option>
            <option value="transmit_blip">Military Transmit Blip</option>
          </select>
        </div>

        <div style="display:flex; flex-direction:column; gap:4px;">
          <label>Closing Radio Squelch / Ack</label>
          <select id="closing_chime">
            <option value="none">None</option>
            <option value="radio_squelch">Subtle Radio Squelch</option>
            <option value="cs_radio_off">CS Comms Release Burst</option>
            <option value="mic_release_click">Mic Release Click</option>
            <option value="cockpit_ack">Two-Tone Cockpit Ack</option>
          </select>
        </div>
      </div>

      <div style="display:flex; align-items:center; gap:8px; margin-top:6px;">
        <input type="checkbox" id="radio_bandpass" style="width:16px; height:16px; cursor:pointer;" onchange="onRadioToggle()">
        <label for="radio_bandpass" style="cursor:pointer; color:#fff;">Enable Military VHF Bandpass (420Hz – 3.4kHz)</label>
      </div>

      <div id="radio_controls" style="display:none; flex-direction:column; gap:8px; padding-left:24px;">
        <div class="slider-row">
          <label>Radio Drive / Saturation <span id="val_radio_drive" class="slider-val">2.2x</span></label>
          <div class="slider-container">
            <input type="range" id="radio_drive" min="1.0" max="4.0" step="0.2" value="2.2" oninput="updateVal('radio_drive', this.value + 'x')">
          </div>
        </div>
        <div class="slider-row">
          <label>RF Background Noise <span id="val_rf_noise" class="slider-val">25%</span></label>
          <div class="slider-container">
            <input type="range" id="rf_noise" min="0" max="0.6" step="0.05" value="0.25" oninput="updateVal('rf_noise', Math.round(this.value/0.6*100) + '%')">
          </div>
        </div>
      </div>

      <div class="card-title" style="margin-top:10px;">4. Save as Voquill Preset</div>
      <div style="display:flex; gap:8px;">
        <input type="text" id="preset_name" placeholder="Preset Name (e.g. British Titan SAS)">
        <button class="btn btn-success" onclick="saveCurrentPreset()" style="white-space:nowrap;">💾 Save Preset</button>
      </div>
    </div>
  </div>

  <!-- TAB 2: SAVED PRESETS -->
  <div id="tab-presets" class="card" style="display:none;">
    <div class="card-title">Saved Voice Presets (Stored in voice_presets.json)</div>
    <div id="presets_list">No presets saved yet.</div>
  </div>

  <!-- TAB 3: FEEDBACK LOG -->
  <div id="tab-feedback" class="card" style="display:none;">
    <div class="card-title">Sample Feedback Reference (Samples 1–27)</div>
    <div style="overflow-x:auto;">
      <table id="feedback_table">
        <thead>
          <tr>
            <th>#</th>
            <th>File</th>
            <th>Base Model</th>
            <th>Style</th>
            <th>Status</th>
            <th>Notes</th>
          </tr>
        </thead>
        <tbody></tbody>
      </table>
    </div>
  </div>

</div>

<script>
  let models = [];

  function switchTab(tab) {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('#tab-tuner, #tab-presets, #tab-feedback').forEach(t => t.style.display = 'none');

    if (tab === 'tuner') {
      document.querySelector('.tab-btn:nth-child(1)').classList.add('active');
      document.getElementById('tab-tuner').style.display = 'grid';
    } else if (tab === 'presets') {
      document.querySelector('.tab-btn:nth-child(2)').classList.add('active');
      document.getElementById('tab-presets').style.display = 'flex';
      loadPresetsList();
    } else if (tab === 'feedback') {
      document.querySelector('.tab-btn:nth-child(3)').classList.add('active');
      document.getElementById('tab-feedback').style.display = 'flex';
      loadFeedbackLog();
    }
  }

  function updateVal(id, val) {
    document.getElementById('val_' + id).innerText = val;
  }

  function resetDspKnobs() {
    document.getElementById('pitch').value = 0;
    document.getElementById('speed').value = 1.0;
    document.getElementById('noise_scale').value = 0.667;
    document.getElementById('sub_bass').value = 0;
    document.getElementById('comb_mix').value = 0;
    document.getElementById('flanger_mix').value = 0;
    document.getElementById('opening_chime').value = 'none';
    document.getElementById('closing_chime').value = 'none';
    document.getElementById('radio_bandpass').checked = false;
    document.getElementById('radio_drive').value = 2.2;
    document.getElementById('rf_noise').value = 0.25;

    updateVal('pitch', '0.0 st');
    updateVal('speed', '1.00x');
    updateVal('noise_scale', '0.67');
    updateVal('sub_bass', '0%');
    updateVal('comb_mix', '0%');
    updateVal('flanger_mix', '0%');
    updateVal('radio_drive', '2.2x');
    updateVal('rf_noise', '25%');

    onRadioToggle();
  }

  function setText(txt) {
    document.getElementById('test_text').value = txt;
  }

  function onRadioToggle() {
    const enabled = document.getElementById('radio_bandpass').checked;
    document.getElementById('radio_controls').style.display = enabled ? 'flex' : 'none';
  }

  function onModelChange() {
    const key = document.getElementById('model_key').value;
    const isMulti = key.includes('libritts');
    document.getElementById('speaker_box').style.display = isMulti ? 'flex' : 'none';
  }

  async function loadModels() {
    const res = await fetch('/api/models');
    models = await res.json();
    const sel = document.getElementById('model_key');
    sel.innerHTML = models.map(m => `<option value="${m.id}">${m.label}</option>`).join('');
    onModelChange();
  }

  async function generateAudio() {
    const btn = document.getElementById('btn_generate');
    const status = document.getElementById('status_msg');
    const audio = document.getElementById('audio_player');

    btn.disabled = true;
    btn.innerText = '⏳ Generating & Rendering Audio...';
    status.innerText = 'Synthesizing with Sherpa-ONNX & applying DSP...';

    const payload = {
      text: document.getElementById('test_text').value,
      model_key: document.getElementById('model_key').value,
      speaker_id: parseInt(document.getElementById('speaker_id').value, 10) || 0,
      speed: parseFloat(document.getElementById('speed').value),
      noise_scale: parseFloat(document.getElementById('noise_scale').value),
      pitch: parseFloat(document.getElementById('pitch').value),
      sub_bass: parseFloat(document.getElementById('sub_bass').value),
      comb_mix: parseFloat(document.getElementById('comb_mix').value),
      flanger_mix: parseFloat(document.getElementById('flanger_mix').value),
      radio_bandpass: document.getElementById('radio_bandpass').checked,
      radio_drive: parseFloat(document.getElementById('radio_drive').value),
      rf_noise: parseFloat(document.getElementById('rf_noise').value),
      opening_chime: document.getElementById('opening_chime').value,
      closing_chime: document.getElementById('closing_chime').value,
    };

    try {
      const res = await fetch('/api/synthesize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });

      if (!res.ok) {
        const err = await res.text();
        throw new Error(err);
      }

      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      audio.src = url;
      audio.style.display = 'block';
      audio.play();
      status.innerHTML = `<span style="color:#34d399;">✓ Playing 2-channel stereo audio in both ears!</span>`;
    } catch (e) {
      status.innerHTML = `<span style="color:#f87171;">Error: ${e.message}</span>`;
    } finally {
      btn.disabled = false;
      btn.innerText = '▶ Generate & Play Audio';
    }
  }

  async function saveCurrentPreset() {
    const name = document.getElementById('preset_name').value.trim();
    if (!name) {
      alert('Please enter a preset name!');
      return;
    }

    const id = name.toLowerCase().replace(/[^a-z0-9]/g, '-');
    const payload = {
      id: id,
      name: name,
      category: 'Custom User Preset',
      description: `Custom Voquill Voice Lab tuned preset for ${name}`,
      model_key: document.getElementById('model_key').value,
      speaker_id: parseInt(document.getElementById('speaker_id').value, 10) || 0,
      speed: parseFloat(document.getElementById('speed').value),
      pitch: parseFloat(document.getElementById('pitch').value),
      sub_bass: parseFloat(document.getElementById('sub_bass').value),
      comb_mix: parseFloat(document.getElementById('comb_mix').value),
      flanger_mix: parseFloat(document.getElementById('flanger_mix').value),
      radio_bandpass: document.getElementById('radio_bandpass').checked,
      radio_drive: parseFloat(document.getElementById('radio_drive').value),
      rf_noise: parseFloat(document.getElementById('rf_noise').value),
      opening_chime: document.getElementById('opening_chime').value,
      closing_chime: document.getElementById('closing_chime').value,
    };

    const res = await fetch('/api/presets', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    if (res.ok) {
      alert(`Preset "${name}" saved successfully to voice_presets.json!`);
      document.getElementById('preset_name').value = '';
    }
  }

  async function loadPresetsList() {
    const res = await fetch('/api/presets');
    const presets = await res.json();
    const container = document.getElementById('presets_list');

    if (!presets || presets.length === 0) {
      container.innerHTML = '<p style="color:var(--text-muted);">No presets saved yet. Tune a voice in the Interactive Tuner and click Save Preset!</p>';
      return;
    }

    container.innerHTML = presets.map(p => `
      <div style="background:rgba(255,255,255,0.03); border:1px solid var(--card-border); padding:12px; border-radius:8px; display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
        <div>
          <div style="font-weight:700; font-size:14px; color:#c7d2fe;">${p.name}</div>
          <div style="font-size:11.5px; color:var(--text-muted);">${p.model_key} | Pitch: ${p.pitch}st | Speed: ${p.speed}x | Radio: ${p.radio_bandpass ? 'Yes' : 'No'}</div>
        </div>
        <div style="display:flex; gap:6px;">
          <button class="btn btn-primary" style="padding:6px 12px; font-size:11.5px;" onclick='loadPresetIntoTuner(${JSON.stringify(p)})'>Load into Tuner</button>
          <button class="btn" style="background:rgba(248,113,113,0.2); color:#f87171; border:1px solid rgba(248,113,113,0.3); padding:6px 10px; font-size:11.5px;" onclick="deletePreset('${p.id}')">Delete</button>
        </div>
      </div>
    `).join('');
  }

  function loadPresetIntoTuner(p) {
    document.getElementById('model_key').value = p.model_key;
    document.getElementById('speaker_id').value = p.speaker_id || 0;
    document.getElementById('speed').value = p.speed;
    document.getElementById('pitch').value = p.pitch;
    document.getElementById('sub_bass').value = p.sub_bass || 0;
    document.getElementById('comb_mix').value = p.comb_mix || 0;
    document.getElementById('flanger_mix').value = p.flanger_mix || 0;
    document.getElementById('radio_bandpass').checked = p.radio_bandpass || false;
    document.getElementById('radio_drive').value = p.radio_drive || 1.0;
    document.getElementById('rf_noise').value = p.rf_noise || 0;
    document.getElementById('opening_chime').value = p.opening_chime || 'none';
    document.getElementById('closing_chime').value = p.closing_chime || 'none';

    updateVal('pitch', p.pitch + ' st');
    updateVal('speed', p.speed + 'x');
    updateVal('sub_bass', Math.round((p.sub_bass||0)*100) + '%');
    updateVal('comb_mix', Math.round((p.comb_mix||0)*100) + '%');
    updateVal('flanger_mix', Math.round((p.flanger_mix||0)*100) + '%');
    updateVal('radio_drive', (p.radio_drive||1.0) + 'x');
    updateVal('rf_noise', Math.round(((p.rf_noise||0)/0.6)*100) + '%');

    onRadioToggle();
    onModelChange();
    switchTab('tuner');
  }

  async function deletePreset(id) {
    if (!confirm('Are you sure you want to delete this preset?')) return;
    await fetch(`/api/presets/${id}`, { method: 'DELETE' });
    loadPresetsList();
  }

  async function loadFeedbackLog() {
    const res = await fetch('/api/samples_log');
    const data = await res.json();
    const tbody = document.querySelector('#feedback_table tbody');

    tbody.innerHTML = (data.samples || []).map(s => {
      let statusClass = 'status-discard';
      if (s.user_status.includes('Keep')) statusClass = 'status-keep';
      else if (s.user_status.includes('Relabel') || s.user_status.includes('Candidate') || s.user_status.includes('Consolidate')) statusClass = 'status-relabel';

      return `
        <tr>
          <td style="font-family:'JetBrains Mono'; font-weight:700;">#${s.id}</td>
          <td style="font-family:'JetBrains Mono'; color:#c7d2fe;">${s.filename}</td>
          <td>${s.base_model}</td>
          <td>${s.style}</td>
          <td class="${statusClass}">${s.user_status}</td>
          <td style="color:#cbd5e1;">${s.user_feedback}</td>
        </tr>
      `;
    }).join('');
  }

  loadModels();
</script>

</body>
</html>
"""

def main():
    port = 8888
    logger.info("Starting Voquill Voice Lab on http://localhost:%d", port)
    uvicorn.run(app, host="0.0.0.0", port=port, log_level="info")

if __name__ == "__main__":
    main()
