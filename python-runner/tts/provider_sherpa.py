"""
Offline neural text-to-speech with sherpa-onnx (Piper / VITS models) and
audio DSP effects.

Pure execution: Rust resolves the voice, downloads its files and sends a
complete TtsRenderRequest. This module keeps no voice catalog, reads no
settings files and downloads nothing.
"""

from __future__ import annotations

from collections import OrderedDict
import gc
import logging
import os
import tempfile
from typing import Any

import numpy as np
import soundfile as sf
from .dsp import (
    apply_custom_dsp,
    apply_dsp_effect,
    to_stereo,
)
from .schemas import (
    CustomDsp,
    TtsRenderRequest,
    TtsSynthesizeResponse,
)

logger = logging.getLogger("voquill.tts.sherpa")

# Cache TTS instances (bounded LRU to prevent memory bloat)
_TTS_INSTANCES: OrderedDict[str, Any] = OrderedDict()
MAX_CACHED_MODELS = 2



def _get_tts_instance(request: TtsRenderRequest) -> Any:
    cache_key = f"{request.model_path}_{request.noise_scale:.3f}"
    if cache_key in _TTS_INSTANCES:
        _TTS_INSTANCES.move_to_end(cache_key)
        return _TTS_INSTANCES[cache_key]

    import sherpa_onnx

    for path in (request.model_path, request.tokens_path, request.data_dir):
        if not os.path.exists(path):
            raise FileNotFoundError(f"Voice file missing: {path}")

    vits_config = sherpa_onnx.OfflineTtsVitsModelConfig(
        model=request.model_path,
        tokens=request.tokens_path,
        data_dir=request.data_dir,
        noise_scale=request.noise_scale,
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
        raise RuntimeError(f"Invalid sherpa-onnx TTS configuration for {request.model_path}")

    tts = sherpa_onnx.OfflineTts(tts_config)

    while len(_TTS_INSTANCES) >= MAX_CACHED_MODELS:
        oldest_key, _ = _TTS_INSTANCES.popitem(last=False)
        logger.info("Evicted cached TTS model: %s to free memory", oldest_key)
        gc.collect()

    _TTS_INSTANCES[cache_key] = tts
    return tts


def run(request: TtsRenderRequest) -> TtsSynthesizeResponse:
    text = request.text.strip()
    if not text:
        raise ValueError("Text to synthesize cannot be empty")

    tts = _get_tts_instance(request)
    logger.info(
        "Rendering %d chars [model=%s, sid=%d, speed=%.2f, dsp=%s]",
        len(text),
        os.path.basename(request.model_path),
        request.speaker_id,
        request.speed,
        request.dsp.kind,
    )
    audio = tts.generate(text, sid=request.speaker_id, speed=request.speed)
    if len(audio.samples) == 0:
        raise RuntimeError("TTS generation produced 0 audio samples")

    samples = np.array(audio.samples, dtype=np.float32)
    dsp = request.dsp
    if isinstance(dsp, CustomDsp):
        processed = apply_custom_dsp(
            samples=samples,
            sample_rate=audio.sample_rate,
            pitch=dsp.pitch,
            sub_bass=dsp.sub_bass,
            comb_mix=dsp.comb_mix,
            flanger_mix=dsp.flanger_mix,
            radio_bandpass=dsp.radio_bandpass,
            radio_drive=dsp.radio_drive,
            rf_noise=dsp.rf_noise,
            opening_chime=dsp.opening_chime,
            closing_chime=dsp.closing_chime,
        )
    else:
        processed = apply_dsp_effect(samples, audio.sample_rate, effect=dsp.effect, pitch=dsp.pitch)

    output_path = request.output_path
    if not output_path:
        fd, output_path = tempfile.mkstemp(suffix=".wav", prefix="voquill_tts_")
        os.close(fd)
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    sf.write(output_path, to_stereo(processed), audio.sample_rate)

    duration_secs = len(processed) / float(audio.sample_rate)
    logger.info("TTS audio saved to %s (%.2fs at %dHz)", output_path, duration_secs, audio.sample_rate)
    return TtsSynthesizeResponse(
        output_path=output_path,
        duration_secs=duration_secs,
        sample_rate=audio.sample_rate,
        provider="sherpa-onnx",
    )
