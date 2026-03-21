import os
import tempfile
import asyncio
from pathlib import Path

from fastapi import FastAPI, UploadFile, File, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import yt_dlp
from faster_whisper import WhisperModel

app = FastAPI()

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

MODEL_SIZE = os.getenv("WHISPER_MODEL", "medium")
print(f"Loading Whisper model: {MODEL_SIZE}")
model = WhisperModel(MODEL_SIZE, device="cpu", compute_type="int8")
print("Whisper model loaded.")


class UrlRequest(BaseModel):
    url: str


def _transcribe(audio_path: str) -> str:
    segments, _ = model.transcribe(audio_path, language="en", beam_size=5)
    return " ".join(seg.text.strip() for seg in segments)


def _download_url(url: str, tmpdir: str):
    ydl_opts = {
        "format": "bestaudio/best",
        "postprocessors": [{"key": "FFmpegExtractAudio", "preferredcodec": "mp3"}],
        "outtmpl": os.path.join(tmpdir, "audio.%(ext)s"),
        "quiet": True,
        "no_warnings": True,
    }
    with yt_dlp.YoutubeDL(ydl_opts) as ydl:
        info = ydl.extract_info(url, download=True)
        title = info.get("title", "")
    audio_file = next(Path(tmpdir).glob("audio.*"))
    return str(audio_file), title


@app.post("/transcribe/url")
async def transcribe_url(req: UrlRequest):
    with tempfile.TemporaryDirectory() as tmpdir:
        try:
            audio_path, title = await asyncio.to_thread(_download_url, req.url, tmpdir)
        except Exception as e:
            raise HTTPException(status_code=400, detail=f"音声の取得に失敗しました: {e}")

        try:
            text = await asyncio.to_thread(_transcribe, audio_path)
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"文字起こしに失敗しました: {e}")

    return {"text": text, "title": title}


@app.post("/transcribe/file")
async def transcribe_file(file: UploadFile = File(...)):
    with tempfile.TemporaryDirectory() as tmpdir:
        audio_path = os.path.join(tmpdir, file.filename or "audio.mp3")
        content = await file.read()
        with open(audio_path, "wb") as f:
            f.write(content)

        try:
            text = await asyncio.to_thread(_transcribe, audio_path)
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"文字起こしに失敗しました: {e}")

    return {"text": text}
