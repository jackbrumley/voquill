"""
Voquill Python Sidecar — capability-based audio processing server.

Architecture:
- Single FastAPI server, started and managed by the Rust app
- Each capability (diarization, VAD, etc.) is a separate module
- Modules register endpoints via a standard pattern
- GET /capabilities returns what's available
- GET /health for lifecycle checks

Adding a new capability:
1. Create a new module directory (e.g., enhancement/)
2. Create a provider file with a run() function
3. Register it in the CAPABILITIES dict in _discover_capabilities()
4. Add dependencies to requirements/<capability>.txt
5. No Rust changes needed — Rust queries /capabilities at startup
"""

from __future__ import annotations

import importlib
import logging
import os
import sys
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

# ── Logging ────────────────────────────────────────────────────────────────
logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s.%(msecs)03d] %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("voquill.server")

# ── Startup state ──────────────────────────────────────────────────────────
# Set by the Rust launcher as an env var so the server knows its home.
RUNNER_BASE_DIR = os.environ.get(
    "VOQUILL_PYTHON_RUNNER_DIR",
    os.path.dirname(os.path.abspath(__file__)),
)


class CapabilityInfo(BaseModel):
    name: str
    provider: str
    description: str


# ── Capability discovery ───────────────────────────────────────────────────
def _discover_capabilities() -> dict[str, dict[str, Any]]:
    """
    Returns a dict mapping capability names to their module info.
    Each entry has:
      - module: the Python module path
      - provider: friendly name for /capabilities
      - description: human-readable description
    """
    return {
        "diarize": {
            "module": "diarization.provider_sherpa",
            "provider": "sherpa-onnx",
            "description": "Speaker diarization (who spoke when) using sherpa-onnx",
        },
        "enhance": {
            "module": "enhancement.provider_noise",
            "provider": "noisereduce",
            "description": "Audio noise reduction using spectral gating (noisereduce)",
        },
        "tts": {
            "module": "tts.provider_sherpa",
            "provider": "sherpa-onnx",
            "description": "Offline text-to-speech synthesis using sherpa-onnx (Piper/VITS)",
        },
    }


def _load_capability_handlers(
    capabilities: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    handlers: dict[str, Any] = {}
    for name, info in capabilities.items():
        try:
            mod = importlib.import_module(info["module"])
            if not hasattr(mod, "run"):
                logger.warning("Capability '%s' has no run() function, skipping", name)
                continue
            if hasattr(mod, "is_available") and not mod.is_available():
                logger.warning(
                    "Capability '%s' is not available (deps missing?), skipping", name
                )
                continue
            handlers[name] = {"module": mod, "info": info}
            logger.info("Loaded capability: %s (%s)", name, info["provider"])
        except ImportError as e:
            logger.warning(
                "Capability '%s' failed to load (%s), skipping", name, e
            )
        except Exception as e:
            logger.warning(
                "Capability '%s' errored during load (%s), skipping", name, e
            )
    return handlers


# ── App lifecycle ──────────────────────────────────────────────────────────
@asynccontextmanager
async def lifespan(app: FastAPI):
    capabilities = _discover_capabilities()
    handlers = _load_capability_handlers(capabilities)
    app.state.handlers = handlers
    app.state.capabilities = [
        CapabilityInfo(
            name=name,
            provider=info["info"]["provider"],
            description=info["info"]["description"],
        )
        for name, info in handlers.items()
    ]
    logger.info(
        "Server ready — %d capabilities loaded", len(app.state.capabilities)
    )
    yield


app = FastAPI(
    title="Voquill Python Runner",
    version="1.0.0",
    lifespan=lifespan,
)


# ── Endpoints ──────────────────────────────────────────────────────────────
@app.get("/health")
async def health():
    return {"status": "ok"}


@app.get("/capabilities")
async def capabilities() -> list[CapabilityInfo]:
    return list(app.state.capabilities)


@app.post("/diarize")
def diarize(body: dict) -> dict:
    handlers = app.state.handlers
    if "diarize" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="Diarization capability not available (sherpa-onnx not installed?)",
        )
    audio_path = body.get("audio_path")
    if not audio_path:
        raise HTTPException(status_code=400, detail="audio_path is required")
    if not os.path.isfile(audio_path):
        raise HTTPException(
            status_code=400, detail=f"audio_path does not exist: {audio_path}"
        )

    cluster_threshold = body.get("cluster_threshold", 0.5)

    mod = handlers["diarize"]["module"]
    try:
        result = mod.run(
            audio_path=audio_path,
            runner_base_dir=RUNNER_BASE_DIR,
            cluster_threshold=cluster_threshold,
        )
        return result.model_dump()
    except Exception as e:
        logger.exception("Diarization failed")
        raise HTTPException(status_code=500, detail=f"Diarization failed: {e}")


@app.post("/enhance")
def enhance(body: dict) -> dict:
    handlers = app.state.handlers
    if "enhance" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="Enhancement capability not available (noisereduce not installed?)",
        )
    audio_path = body.get("audio_path")
    if not audio_path:
        raise HTTPException(status_code=400, detail="audio_path is required")
    if not os.path.isfile(audio_path):
        raise HTTPException(
            status_code=400, detail=f"audio_path does not exist: {audio_path}"
        )

    noise_reduction_strength = body.get("noise_reduction_strength", 0.7)

    mod = handlers["enhance"]["module"]
    try:
        result = mod.run(
            audio_path=audio_path,
            runner_base_dir=RUNNER_BASE_DIR,
            noise_reduction_strength=noise_reduction_strength,
        )
        return result.model_dump()
    except Exception as e:
        logger.exception("Enhancement failed")
        raise HTTPException(status_code=500, detail=f"Enhancement failed: {e}")


@app.get("/tts/models")
def get_tts_models() -> list[dict]:
    handlers = app.state.handlers
    if "tts" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="TTS capability not available (sherpa-onnx not installed?)",
        )
    mod = handlers["tts"]["module"]
    try:
        models = mod.get_available_models()
        return [m.model_dump() for m in models]
    except Exception as e:
        logger.exception("Failed to query TTS models")
        raise HTTPException(status_code=500, detail=f"Failed to query TTS models: {e}")


@app.get("/tts/voices")
def get_tts_voices() -> list[dict]:
    handlers = app.state.handlers
    if "tts" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="TTS capability not available (sherpa-onnx not installed?)",
        )
    mod = handlers["tts"]["module"]
    try:
        voices = mod.get_available_voices(RUNNER_BASE_DIR)
        return [v.model_dump() for v in voices]
    except Exception as e:
        logger.exception("Failed to query TTS voices")
        raise HTTPException(status_code=500, detail=f"Failed to query TTS voices: {e}")


@app.post("/tts/synthesize")
def synthesize_tts(body: dict) -> dict:
    handlers = app.state.handlers
    if "tts" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="TTS capability not available (sherpa-onnx not installed?)",
        )
    text = body.get("text")
    if not text:
        raise HTTPException(status_code=400, detail="text is required")

    voice_id = body.get("voice_id", "titan-mech")
    speed = body.get("speed", 1.0)
    effect = body.get("effect")
    pitch = body.get("pitch", 0.0)
    output_path = body.get("output_path")

    mod = handlers["tts"]["module"]
    try:
        result = mod.run(
            text=text,
            voice_id=voice_id,
            speed=speed,
            effect=effect,
            pitch=pitch,
            output_path=output_path,
            runner_base_dir=RUNNER_BASE_DIR,
        )
        return result.model_dump()
    except Exception as e:
        logger.exception("TTS synthesis failed")
        raise HTTPException(status_code=500, detail=f"TTS synthesis failed: {e}")


@app.post("/tts/synthesize_custom")
def synthesize_custom_tts(body: dict) -> dict:
    handlers = app.state.handlers
    if "tts" not in handlers:
        raise HTTPException(
            status_code=501,
            detail="TTS capability not available (sherpa-onnx not installed?)",
        )
    text = body.get("text")
    if not text:
        raise HTTPException(status_code=400, detail="text is required")

    model_key = body.get("model_key", "piper-en_GB-northern_english_male-medium")
    speaker_id = body.get("speaker_id", 0)
    speed = body.get("speed", 1.0)
    noise_scale = body.get("noise_scale", 0.667)
    pitch = body.get("pitch", 0.0)
    sub_bass = body.get("sub_bass", 0.0)
    comb_mix = body.get("comb_mix", 0.0)
    flanger_mix = body.get("flanger_mix", 0.0)
    radio_bandpass = body.get("radio_bandpass", False)
    radio_drive = body.get("radio_drive", 1.0)
    rf_noise = body.get("rf_noise", 0.0)
    opening_chime = body.get("opening_chime", "none")
    closing_chime = body.get("closing_chime", "none")
    output_path = body.get("output_path")

    mod = handlers["tts"]["module"]
    try:
        result = mod.run_custom(
            text=text,
            model_key=model_key,
            speaker_id=speaker_id,
            speed=speed,
            noise_scale=noise_scale,
            pitch=pitch,
            sub_bass=sub_bass,
            comb_mix=comb_mix,
            flanger_mix=flanger_mix,
            radio_bandpass=radio_bandpass,
            radio_drive=radio_drive,
            rf_noise=rf_noise,
            opening_chime=opening_chime,
            closing_chime=closing_chime,
            output_path=output_path,
            runner_base_dir=RUNNER_BASE_DIR,
        )
        return result.model_dump()
    except Exception as e:
        logger.exception("Custom TTS synthesis failed")
        raise HTTPException(status_code=500, detail=f"Custom TTS synthesis failed: {e}")


# ── Entry point (used when Rust spawns the server) ─────────────────────────
def main():
    port = int(os.environ.get("VOQUILL_PORT", "9000"))
    logger.info("Starting on port %d (base_dir=%s)", port, RUNNER_BASE_DIR)
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")


if __name__ == "__main__":
    main()