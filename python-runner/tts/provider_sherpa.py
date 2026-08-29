"""
Offline neural Text-to-Speech (TTS) using sherpa-onnx (Piper / VITS models)
with authentic audio DSP personality profiles.
Runs on CPU in <100ms with zero PyTorch dependency.
"""

from __future__ import annotations

import json
import logging
import os
import tarfile
import tempfile
import urllib.request
from typing import Any, Dict, List

import numpy as np
import soundfile as sf
from .dsp import (
    apply_custom_dsp,
    apply_dsp_effect,
    to_stereo,
)
from .schemas import (
    BaseVoiceModelInfo,
    TtsSynthesizeResponse,
    VoicePersonaInfo,
)

logger = logging.getLogger("voquill.tts.sherpa")

# Neural models available for download and caching
BASE_MODELS: Dict[str, Dict[str, Any]] = {
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
    "piper-en_US-lessac-low": {
        "label": "🎙️ Lessac (Clear Studio Female)",
        "archive": "vits-piper-en_US-lessac-low",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-lessac-low.tar.bz2",
        "model_file": "en_US-lessac-low.onnx",
        "is_multi_speaker": False,
    },
    "piper-en_US-libritts_r-medium": {
        "label": "🎭 LibriTTS-R Multi-Speaker (904 Speakers)",
        "archive": "vits-piper-en_US-libritts_r-medium",
        "url": "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-libritts_r-medium.tar.bz2",
        "model_file": "en_US-libritts_r-medium.onnx",
        "is_multi_speaker": True,
    },
}

# Curated Voice Personas with expertly tuned DSP & acoustic presets
PERSONA_CATALOG: Dict[str, Dict[str, Any]] = {
    "tactical-comms": {
        "id": "tactical-comms",
        "name": "Tactical Comms",
        "persona": "SAS Tactical Operator",
        "category": "Military & Tactical",
        "engine": "piper",
        "description": "Northern English military comms with authentic VHF bandpass filtering, tactical overdrive, and tail squelch.",
        "base_model": "piper-en_GB-northern_english_male-medium",
        "default_effect": "radio",
        "default_pitch": 0.0,
        "default_speed": 1.05,
    },
    "titan-mech": {
        "id": "titan-mech",
        "name": "Titan Mech",
        "persona": "Armored Cockpit AI",
        "category": "Sci-Fi & Gaming",
        "engine": "piper",
        "description": "Deep, authoritative pilot system with metallic armored chassis resonance and sub-bass weight.",
        "base_model": "piper-en_US-ryan-low",
        "default_effect": "mech",
        "default_pitch": -4.0,
        "default_speed": 0.95,
    },
    "nanosuit": {
        "id": "nanosuit",
        "name": "Nanosuit AI",
        "persona": "Tactical Combat Exosuit",
        "category": "Sci-Fi & Gaming",
        "engine": "piper",
        "description": "Cybernetic combat armor system with synthetic pitch modulation and power-shield resonance.",
        "base_model": "piper-en_US-ryan-low",
        "default_effect": "mech",
        "default_pitch": -2.0,
        "default_speed": 1.0,
    },
    "glados": {
        "id": "glados",
        "name": "GLaDOS AI",
        "persona": "Iconic Robot AI",
        "category": "Sci-Fi & Gaming",
        "engine": "piper",
        "description": "Iconic robotic AI with distinctive robotic inflections and dry demeanor.",
        "base_model": "piper-en_US-glados",
        "default_effect": "clean",
        "default_pitch": 0.0,
        "default_speed": 1.0,
    },
    "cyberpunk-eva": {
        "id": "cyberpunk-eva",
        "name": "Cyberpunk EVA",
        "persona": "Holographic Ship AI",
        "category": "Sci-Fi & Gaming",
        "engine": "piper",
        "description": "Futuristic spacecraft computer with holographic bridge reflections and crystal air clarity.",
        "base_model": "piper-en_US-amy-low",
        "default_effect": "eva",
        "default_pitch": 1.0,
        "default_speed": 1.0,
    },
    "flight-deck": {
        "id": "flight-deck",
        "name": "Flight Deck ATC",
        "persona": "British Flight Controller",
        "category": "Aviation & Simulation",
        "engine": "piper",
        "description": "Crisp British aviation air traffic control / cockpit automated warning system.",
        "base_model": "piper-en_GB-southern_english_female-low",
        "default_effect": "flight_deck",
        "default_pitch": 0.0,
        "default_speed": 1.0,
    },
    "nova-studio": {
        "id": "nova-studio",
        "name": "Nova Studio (Female)",
        "persona": "Clean Studio Female",
        "category": "Studio & Natural",
        "engine": "piper",
        "description": "Natural, crystal-clear studio narration voice with zero acoustic coloration.",
        "base_model": "piper-en_US-lessac-low",
        "default_effect": "clean",
        "default_pitch": 0.0,
        "default_speed": 1.0,
    },
    "apex-studio": {
        "id": "apex-studio",
        "name": "Apex Studio (Male)",
        "persona": "Authoritative Studio Male",
        "category": "Studio & Natural",
        "engine": "piper",
        "description": "Clean, warm, and natural male studio voice for desktop automation and productivity.",
        "base_model": "piper-en_US-ryan-low",
        "default_effect": "clean",
        "default_pitch": 0.0,
        "default_speed": 1.0,
    },
}

# Legacy voice ID mappings for backwards compatibility
LEGACY_ALIASES: Dict[str, str] = {
    "piper-en_US-amy-low": "cyberpunk-eva",
    "piper-en_US-glados": "glados",
    "piper-en_US-ryan-low": "titan-mech",
    "piper-en_GB-southern_english_female-low": "flight-deck",
    "piper-en_US-arctic-medium": "tactical-comms",
    "piper-en_US-lessac-low": "nova-studio",
}

# Cache TTS instances so repeated calls reuse initialized models
_TTS_INSTANCES: Dict[str, Any] = {}


def _ensure_base_model(model_key: str, runner_base_dir: str) -> Dict[str, str]:
    meta = BASE_MODELS.get(model_key)
    if not meta:
        raise ValueError(f"Unknown neural model key '{model_key}'")

    models_dir = os.path.join(runner_base_dir, "models", "tts")
    os.makedirs(models_dir, exist_ok=True)

    voice_dir = os.path.join(models_dir, meta["archive"])
    model_path = os.path.join(voice_dir, meta["model_file"])
    tokens_path = os.path.join(voice_dir, "tokens.txt")
    data_dir = os.path.join(voice_dir, "espeak-ng-data")

    if (
        os.path.exists(model_path)
        and os.path.exists(tokens_path)
        and os.path.exists(data_dir)
    ):
        return {
            "model_path": model_path,
            "tokens_path": tokens_path,
            "data_dir": data_dir,
        }

    archive_name = f"{meta['archive']}.tar.bz2"
    archive_path = os.path.join(models_dir, archive_name)

    logger.info(
        "Downloading TTS voice model %s from %s...", model_key, meta["url"]
    )
    urllib.request.urlretrieve(meta["url"], archive_path)

    logger.info("Extracting TTS voice model %s...", model_key)
    with tarfile.open(archive_path, "r:bz2") as tar:
        tar.extractall(path=models_dir)

    if os.path.exists(archive_path):
        os.remove(archive_path)

    if not (
        os.path.exists(model_path)
        and os.path.exists(tokens_path)
        and os.path.exists(data_dir)
    ):
        raise FileNotFoundError(
            f"Voice files for '{model_key}' not found after extraction in {voice_dir}"
        )

    logger.info("TTS voice model %s ready at %s", model_key, voice_dir)
    return {
        "model_path": model_path,
        "tokens_path": tokens_path,
        "data_dir": data_dir,
    }


def _get_tts_instance(
    model_key: str,
    runner_base_dir: str,
    noise_scale: float = 0.667,
    noise_scale_w: float = 0.8,
    length_scale: float = 1.0,
) -> Any:
    global _TTS_INSTANCES
    cache_key = f"{model_key}_{noise_scale}_{length_scale}"
    if cache_key in _TTS_INSTANCES:
        return _TTS_INSTANCES[cache_key]

    import sherpa_onnx

    paths = _ensure_base_model(model_key, runner_base_dir)

    vits_config = sherpa_onnx.OfflineTtsVitsModelConfig(
        model=paths["model_path"],
        tokens=paths["tokens_path"],
        data_dir=paths["data_dir"],
        noise_scale=noise_scale,
        noise_scale_w=noise_scale_w,
        length_scale=length_scale,
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
        raise RuntimeError(
            f"Invalid sherpa-onnx TTS configuration for model {model_key}"
        )

    tts = sherpa_onnx.OfflineTts(tts_config)
    _TTS_INSTANCES[cache_key] = tts
    return tts


def get_available_models() -> List[BaseVoiceModelInfo]:
    return [
        BaseVoiceModelInfo(
            id=k,
            label=v["label"],
            is_multi_speaker=v.get("is_multi_speaker", False),
        )
        for k, v in BASE_MODELS.items()
    ]


def get_available_voices(runner_base_dir: str) -> list[VoicePersonaInfo]:
    models_dir = os.path.join(runner_base_dir, "models", "tts")
    results: list[VoicePersonaInfo] = []

    # 1. Built-in personas
    for pid, persona in PERSONA_CATALOG.items():
        base_key = persona["base_model"]
        base_meta = BASE_MODELS.get(base_key, {})
        archive = base_meta.get("archive", "")
        model_file = base_meta.get("model_file", "")

        voice_dir = os.path.join(models_dir, archive)
        model_path = os.path.join(voice_dir, model_file)
        is_ready = os.path.exists(model_path)

        results.append(
            VoicePersonaInfo(
                id=pid,
                name=persona["name"],
                persona=persona["persona"],
                category=persona["category"],
                engine=persona["engine"],
                description=persona["description"],
                is_ready=is_ready,
                default_effect=persona.get("default_effect"),
                default_pitch=persona.get("default_pitch"),
                default_speed=persona.get("default_speed"),
            )
        )

    # 2. User saved presets from voice_presets.json
    presets_file = os.path.join(runner_base_dir, "voice_presets.json")
    if os.path.exists(presets_file):
        try:
            with open(presets_file, "r") as f:
                user_presets = json.load(f)
                for p in user_presets:
                    base_key = p.get("model_key", "piper-en_US-ryan-low")
                    base_meta = BASE_MODELS.get(base_key, {})
                    archive = base_meta.get("archive", "")
                    model_file = base_meta.get("model_file", "")
                    voice_dir = os.path.join(models_dir, archive)
                    model_path = os.path.join(voice_dir, model_file)
                    is_ready = os.path.exists(model_path)

                    results.append(
                        VoicePersonaInfo(
                            id=p["id"],
                            name=p["name"],
                            persona="Custom User Preset",
                            category="Custom Presets",
                            engine="piper",
                            description=p.get("description", "Custom Voice Lab Preset"),
                            is_ready=is_ready,
                            default_effect="custom",
                            default_pitch=p.get("pitch", 0.0),
                            default_speed=p.get("speed", 1.0),
                        )
                    )
        except Exception as e:
            logger.warning("Could not read voice_presets.json: %s", e)

    return results


def run_custom(
    text: str,
    model_key: str,
    speaker_id: int = 0,
    speed: float = 1.0,
    noise_scale: float = 0.667,
    pitch: float = 0.0,
    sub_bass: float = 0.0,
    comb_mix: float = 0.0,
    flanger_mix: float = 0.0,
    radio_bandpass: bool = False,
    radio_drive: float = 1.0,
    rf_noise: float = 0.0,
    opening_chime: str = "none",
    closing_chime: str = "none",
    output_path: str | None = None,
    runner_base_dir: str = ".",
) -> TtsSynthesizeResponse:
    clean_text = text.strip()
    if not clean_text:
        raise ValueError("Text to synthesize cannot be empty")

    if model_key not in BASE_MODELS:
        model_key = "piper-en_GB-northern_english_male-medium"

    speed = max(0.5, min(2.0, float(speed)))
    tts = _get_tts_instance(model_key, runner_base_dir, noise_scale=noise_scale)

    logger.info(
        "Custom synthesis (%d chars) [model=%s, sid=%d, speed=%.2f, pitch=%.1f]...",
        len(clean_text),
        model_key,
        speaker_id,
        speed,
        pitch,
    )
    audio = tts.generate(clean_text, sid=speaker_id, speed=speed)

    if len(audio.samples) == 0:
        raise RuntimeError("TTS generation produced 0 audio samples")

    raw_samples = np.array(audio.samples, dtype=np.float32)

    processed = apply_custom_dsp(
        samples=raw_samples,
        sample_rate=audio.sample_rate,
        pitch=pitch,
        sub_bass=sub_bass,
        comb_mix=comb_mix,
        flanger_mix=flanger_mix,
        radio_bandpass=radio_bandpass,
        radio_drive=radio_drive,
        rf_noise=rf_noise,
        opening_chime=opening_chime,
        closing_chime=closing_chime,
    )

    if not output_path:
        fd, temp_out = tempfile.mkstemp(suffix=".wav", prefix="voquill_tts_")
        os.close(fd)
        output_path = temp_out

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    stereo_samples = to_stereo(processed)
    sf.write(output_path, stereo_samples, audio.sample_rate)

    duration_secs = len(processed) / float(audio.sample_rate)
    return TtsSynthesizeResponse(
        output_path=output_path,
        duration_secs=duration_secs,
        sample_rate=audio.sample_rate,
        provider="sherpa-onnx",
    )


def run(
    text: str,
    voice_id: str,
    speed: float = 1.0,
    effect: str | None = None,
    pitch: float = 0.0,
    output_path: str | None = None,
    runner_base_dir: str = ".",
) -> TtsSynthesizeResponse:
    clean_text = text.strip()
    if not clean_text:
        raise ValueError("Text to synthesize cannot be empty")

    # Check if voice_id is a custom user preset from voice_presets.json
    presets_file = os.path.join(runner_base_dir, "voice_presets.json")
    if os.path.exists(presets_file):
        try:
            with open(presets_file, "r") as f:
                user_presets = json.load(f)
                for p in user_presets:
                    if p["id"] == voice_id:
                        return run_custom(
                            text=clean_text,
                            model_key=p.get("model_key", "piper-en_US-ryan-low"),
                            speaker_id=p.get("speaker_id", 0),
                            speed=speed if abs(speed - 1.0) > 0.01 else p.get("speed", 1.0),
                            noise_scale=p.get("noise_scale", 0.667),
                            pitch=pitch if abs(pitch) > 0.01 else p.get("pitch", 0.0),
                            sub_bass=p.get("sub_bass", 0.0),
                            comb_mix=p.get("comb_mix", 0.0),
                            flanger_mix=p.get("flanger_mix", 0.0),
                            radio_bandpass=p.get("radio_bandpass", False),
                            radio_drive=p.get("radio_drive", 1.0),
                            rf_noise=p.get("rf_noise", 0.0),
                            opening_chime=p.get("opening_chime", "none"),
                            closing_chime=p.get("closing_chime", "none"),
                            output_path=output_path,
                            runner_base_dir=runner_base_dir,
                        )
        except Exception as e:
            logger.warning("Error checking voice presets in run: %s", e)

    # Resolve legacy aliases if passed
    canonical_id = LEGACY_ALIASES.get(voice_id, voice_id)
    persona = PERSONA_CATALOG.get(canonical_id)

    sid = 0
    if persona:
        base_model_key = persona["base_model"]
        sid = persona.get("sid", 0)
        active_effect = (
            effect
            if (effect and effect != "default" and effect != "clean")
            else persona["default_effect"]
        )
        active_pitch = (
            pitch
            if abs(pitch) > 0.01
            else persona.get("default_pitch", 0.0)
        )
        active_speed = (
            speed
            if abs(speed - 1.0) > 0.01
            else persona.get("default_speed", 1.0)
        )
    elif voice_id in BASE_MODELS:
        base_model_key = voice_id
        active_effect = effect or "clean"
        active_pitch = pitch
        active_speed = speed
    else:
        base_model_key = "piper-en_GB-northern_english_male-medium"
        active_effect = effect or "radio"
        active_pitch = pitch
        active_speed = speed

    active_speed = max(0.5, min(2.0, float(active_speed)))
    tts = _get_tts_instance(base_model_key, runner_base_dir)

    logger.info(
        "Synthesizing text (%d chars) with persona '%s' [model=%s, sid=%d, speed=%.2f, effect='%s', pitch=%.1f]...",
        len(clean_text),
        canonical_id,
        base_model_key,
        sid,
        active_speed,
        active_effect,
        active_pitch,
    )
    audio = tts.generate(clean_text, sid=sid, speed=active_speed)

    if len(audio.samples) == 0:
        raise RuntimeError("TTS generation produced 0 audio samples")

    raw_samples = np.array(audio.samples, dtype=np.float32)

    processed_samples = apply_dsp_effect(
        raw_samples,
        audio.sample_rate,
        effect=active_effect,
        pitch=active_pitch,
    )

    if not output_path:
        fd, temp_out = tempfile.mkstemp(suffix=".wav", prefix="voquill_tts_")
        os.close(fd)
        output_path = temp_out

    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    stereo_samples = to_stereo(processed_samples)
    sf.write(output_path, stereo_samples, audio.sample_rate)

    duration_secs = len(processed_samples) / float(audio.sample_rate)
    logger.info(
        "TTS audio saved to %s (%.2fs at %dHz, effect=%s)",
        output_path,
        duration_secs,
        audio.sample_rate,
        active_effect,
    )

    return TtsSynthesizeResponse(
        output_path=output_path,
        duration_secs=duration_secs,
        sample_rate=audio.sample_rate,
        provider="sherpa-onnx",
    )
