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


class TtsSynthesizeRequest(BaseModel):
    text: str
    voice_id: str
    speed: float = 1.0
    output_path: Optional[str] = None


class TtsSynthesizeResponse(BaseModel):
    output_path: str
    duration_secs: float
    sample_rate: int
    provider: str
