from pydantic import BaseModel


class Segment(BaseModel):
    speaker: str | None
    text: str
    start_sec: float | None
    end_sec: float | None


class DiarizeRequest(BaseModel):
    audio_path: str


class DiarizeResponse(BaseModel):
    segments: list[Segment]
    provider: str