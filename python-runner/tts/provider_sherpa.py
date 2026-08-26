"""
Offline neural Text-to-Speech (TTS) using sherpa-onnx (Piper / VITS models).
Runs on CPU in <100ms with zero PyTorch dependency.
"""

from __future__ import annotations

import logging
import os
import tarfile
import tempfile
import urllib.request
from typing import Any, Dict

import numpy as np
import soundfile as sf
from .dsp import apply_dsp_effect
from .schemas import TtsSynthesizeResponse, VoicePersonaInfo

logger = logging.getLogger("voquill.tts.sherpa")

VOICE_CATALOG: Dict[str, Dict[str, str]] = {
    "piper-en_US-amy-low": {
        "id": "piper-en_US-amy-low",
        "name": "Cyberpunk EVA",
        "persona": "Crisp Sci-Fi Female AI",
        "category": "Sci-Fi / Cockpit",
        "engine": "piper",
        "description": "Futuristic, calm, and intelligent ship computer voice.",
        "archive": "vits-piper-en_US-amy-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-low.tar.bz2",
        "model_file": "en_US-amy-low.onnx",
    },
    "piper-en_US-glados": {
        "id": "piper-en_US-glados",
        "name": "GLaDOS AI",
        "persona": "Iconic Robot AI",
        "category": "Robotic / AI",
        "engine": "piper",
        "description": "Iconic robotic AI with distinctive robotic inflections.",
        "archive": "vits-piper-en_US-glados",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-glados.tar.bz2",
        "model_file": "en_US-glados.onnx",
    },
    "piper-en_US-ryan-low": {
        "id": "piper-en_US-ryan-low",
        "name": "Titan Mech",
        "persona": "Deep Cockpit Male",
        "category": "Sci-Fi / Cockpit",
        "engine": "piper",
        "description": "Authoritative, deep mechanical pilot system voice.",
        "archive": "vits-piper-en_US-ryan-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-ryan-low.tar.bz2",
        "model_file": "en_US-ryan-low.onnx",
    },
    "piper-en_GB-southern_english_female-low": {
        "id": "piper-en_GB-southern_english_female-low",
        "name": "Aero Cockpit",
        "persona": "British Flight Deck AI",
        "category": "Sci-Fi / Cockpit",
        "engine": "piper",
        "description": "Crisp British ATC / flight deck automated system.",
        "archive": "vits-piper-en_GB-southern_english_female-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-southern_english_female-low.tar.bz2",
        "model_file": "en_GB-southern_english_female-low.onnx",
    },
    "piper-en_US-arctic-medium": {
        "id": "piper-en_US-arctic-medium",
        "name": "Tactical Radio",
        "persona": "Military Radio Comms",
        "category": "Tactical Military",
        "engine": "piper",
        "description": "Tactical, direct military comms channel tone.",
        "archive": "vits-piper-en_US-arctic-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-arctic-medium.tar.bz2",
        "model_file": "en_US-arctic-medium.onnx",
    },
    "piper-en_US-lessac-low": {
        "id": "piper-en_US-lessac-low",
        "name": "Nova Studio",
        "persona": "Clear Studio Voice",
        "category": "Realistic / Studio",
        "engine": "piper",
        "description": "Clean, human-grade natural studio voice.",
        "archive": "vits-piper-en_US-lessac-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-lessac-low.tar.bz2",
        "model_file": "en_US-lessac-low.onnx",
    },
}

# Cache TTS instances so repeated calls reuse initialized models
_TTS_INSTANCES: Dict[str, Any] = {}


def _ensure_voice_model(voice_id: str, runner_base_dir: str) -> Dict[str, str]:
    meta = VOICE_CATALOG.get(voice_id)
    if not meta:
        raise ValueError(f"Unknown voice ID '{voice_id}'")

    models_dir = os.path.join(runner_base_dir, "models", "tts")
    os.makedirs(models_dir, exist_ok=True)

    voice_dir = os.path.join(models_dir, meta["archive"])
    model_path = os.path.join(voice_dir, meta["model_file"])
    tokens_path = os.path.join(voice_dir, "tokens.txt")
    data_dir = os.path.join(voice_dir, "espeak-ng-data")

    if os.path.exists(model_path) and os.path.exists(tokens_path) and os.path.exists(data_dir):
        return {
            "model_path": model_path,
            "tokens_path": tokens_path,
            "data_dir": data_dir,
        }

    archive_name = f"{meta['archive']}.tar.bz2"
    archive_path = os.path.join(models_dir, archive_name)

    logger.info("Downloading TTS voice model %s from %s...", voice_id, meta["url"])
    urllib.request.urlretrieve(meta["url"], archive_path)

    logger.info("Extracting TTS voice model %s...", voice_id)
    with tarfile.open(archive_path, "r:bz2") as tar:
        tar.extractall(path=models_dir)

    if os.path.exists(archive_path):
        os.remove(archive_path)

    if not (os.path.exists(model_path) and os.path.exists(tokens_path) and os.path.exists(data_dir)):
        raise FileNotFoundError(
            f"Voice files for '{voice_id}' not found after extraction in {voice_dir}"
        )

    logger.info("TTS voice model %s ready at %s", voice_id, voice_dir)
    return {
        "model_path": model_path,
        "tokens_path": tokens_path,
        "data_dir": data_dir,
    }


def _get_tts_instance(voice_id: str, runner_base_dir: str) -> Any:
    global _TTS_INSTANCES
    if voice_id in _TTS_INSTANCES:
        return _TTS_INSTANCES[voice_id]

    import sherpa_onnx

    paths = _ensure_voice_model(voice_id, runner_base_dir)

    vits_config = sherpa_onnx.OfflineTtsVitsModelConfig(
        model=paths["model_path"],
        tokens=paths["tokens_path"],
        data_dir=paths["data_dir"],
        noise_scale=0.667,
        noise_scale_w=0.8,
        length_scale=1.0,
    )
    model_config = sherpa_onnx.OfflineTtsModelConfig(
        vits=vits_config,
        num_threads=2,
        debug=0,
        provider="cpu",
    )
    tts_config = sherpa_onnx.OfflineTtsConfig(
        model=model_config,
        rule_fsts="",
        max_num_sentences=1,
    )

    if not tts_config.validate():
        raise RuntimeError(f"Invalid sherpa-onnx TTS configuration for voice {voice_id}")

    tts = sherpa_onnx.OfflineTts(tts_config)
    _TTS_INSTANCES[voice_id] = tts
    return tts


def get_available_voices(runner_base_dir: str) -> list[VoicePersonaInfo]:
    models_dir = os.path.join(runner_base_dir, "models", "tts")
    results: list[VoicePersonaInfo] = []

    for vid, meta in VOICE_CATALOG.items():
        voice_dir = os.path.join(models_dir, meta["archive"])
        model_path = os.path.join(voice_dir, meta["model_file"])
        is_ready = os.path.exists(model_path)
        results.append(
            VoicePersonaInfo(
                id=vid,
                name=meta["name"],
                persona=meta["persona"],
                category=meta["category"],
                engine=meta["engine"],
                description=meta["description"],
                is_ready=is_ready,
            )
        )
    return results


def run(
    text: str,
    voice_id: str,
    speed: float = 1.0,
    effect: str = "clean",
    pitch: float = 0.0,
    output_path: str | None = None,
    runner_base_dir: str = ".",
) -> TtsSynthesizeResponse:
    clean_text = text.strip()
    if not clean_text:
        raise ValueError("Text to synthesize cannot be empty")

    if voice_id not in VOICE_CATALOG:
        # Fall back to default female AI if unknown
        voice_id = "piper-en_US-amy-low"

    speed = max(0.5, min(2.0, float(speed)))

    tts = _get_tts_instance(voice_id, runner_base_dir)

    logger.info("Synthesizing text (%d chars) with voice '%s' (speed=%.2f, effect='%s', pitch=%.1f)...", len(clean_text), voice_id, speed, effect, pitch)
    audio = tts.generate(clean_text, sid=0, speed=speed)

    if len(audio.samples) == 0:
        raise RuntimeError("TTS generation produced 0 audio samples")

    raw_samples = np.array(audio.samples, dtype=np.float32)

    # Apply audio DSP effect (Titan Mech / Tactical Radio / Cyberpunk EVA / Pitch Shift)
    processed_samples = apply_dsp_effect(raw_samples, audio.sample_rate, effect=effect, pitch=pitch)

    if not output_path:
        fd, temp_out = tempfile.mkstemp(suffix=".wav", prefix="voquill_tts_")
        os.close(fd)
        output_path = temp_out

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    sf.write(output_path, processed_samples, audio.sample_rate)

    duration_secs = len(processed_samples) / float(audio.sample_rate)
    logger.info("TTS audio saved to %s (%.2fs at %dHz, effect=%s)", output_path, duration_secs, audio.sample_rate, effect)

    return TtsSynthesizeResponse(
        output_path=output_path,
        duration_secs=duration_secs,
        sample_rate=audio.sample_rate,
        provider="sherpa-onnx",
    )
