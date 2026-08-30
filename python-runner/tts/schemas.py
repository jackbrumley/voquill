from __future__ import annotations

from typing import Optional
from pydantic import BaseModel


class VoicePersonaInfo(BaseModel):
    id: str
    name: str
    persona: str
    category: str
    engine: str
    description: str
    is_ready: bool
    default_effect: Optional[str] = None
    default_pitch: Optional[float] = None
    default_speed: Optional[float] = None


class BaseVoiceModelInfo(BaseModel):
    id: str
    label: str
    is_multi_speaker: bool = False
    is_ready: bool = False


class TtsSynthesizeRequest(BaseModel):
    text: str
    voice_id: str
    speed: float = 1.0
    effect: Optional[str] = "clean"
    pitch: Optional[float] = 0.0
    output_path: Optional[str] = None


class CustomTtsSynthesizeRequest(BaseModel):
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
    output_path: Optional[str] = None


class VoicePresetSchema(BaseModel):
    id: str
    name: str
    category: str
    description: str
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


class TtsSynthesizeResponse(BaseModel):
    output_path: str
    duration_secs: float
    sample_rate: int
    provider: str
