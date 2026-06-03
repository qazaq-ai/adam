#!/usr/bin/env python3
"""
Scrape kaz-tili.kz audio + transcript pairs into a flat corpus.

Walks every .htm linked from the site's index page, extracts every
<a href="./wav2/*.mp3"> reference, pulls the Kazakh text wrapped in
<span class=kaz>...</span> that immediately follows the audio link,
strips HTML / Russian gloss, downloads the MP3 with ffmpeg-converts
to 16 kHz mono WAV, and writes a JSONL manifest pair-by-pair.

Output (under data/kaz_tili/):
  audio/<label>.wav          — 16 kHz mono pure PCM
  transcripts.jsonl          — one JSON per audio
    { "label": "...", "audio_path": "...", "transcript": "...",
      "source_page": "...", "audio_url": "..." }

Quality gate: drops MP3s shorter than 0.10 s (likely buttons / icons),
empty transcripts, and transcripts with no native Kazakh letters.

`build-time tool`: uses ffmpeg externally. Not part of the runtime
pure-Rust pledge.
"""

import html
import json
import os
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = "https://www.kaz-tili.kz"
INDEX = f"{ROOT}/index.htm"
OUT = Path("data/kaz_tili")
AUDIO_DIR = OUT / "audio"
MANIFEST = OUT / "transcripts.jsonl"

# Kazakh-Cyrillic letter set — drop transcripts that contain none.
KAZ_LETTERS = set("абвгдежзийклмнопрстуфхчшыюяҚқҒғҺһҢңӘәҰұӨөҮүІі")

# HTML inline-text regex: pull text inside <span class=kaz>...</span>
# (kaz-tili uses both class=kaz and class="kaz"; tolerate either).
SPAN_KAZ = re.compile(
    r'<span\s+class\s*=\s*"?kaz"?\s*>(.*?)</span>',
    re.IGNORECASE | re.DOTALL,
)
# Russian gloss is wrapped in <span class=se>...</span> — drop it.
SPAN_SE = re.compile(
    r'<span\s+class\s*=\s*"?se"?\s*>.*?</span>',
    re.IGNORECASE | re.DOTALL,
)
# Generic tag stripper used after gloss removal.
TAG = re.compile(r"<[^>]+>")
# Audio links: <a href="./wav2/<file>.mp3">
AUDIO_HREF = re.compile(
    r'<a\s+href\s*=\s*"\.?/?wav2?/([^"]+\.(?:mp3|wav|ogg))"[^>]*>(.*?)</a>',
    re.IGNORECASE | re.DOTALL,
)
# Sub-page links — both flat (`foo.htm`) and one-level-down
# (`./folder/foo.htm` or `folder/foo.htm`).  Excludes anchors
# (`#…`) and external schemes.
SUB_HREF = re.compile(
    r'href\s*=\s*"(\.?/?[a-z0-9_/]+\.htm)"',
    re.IGNORECASE,
)


def fetch(url: str, timeout: int = 30) -> bytes:
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": "adam-kazakh-corpus-scraper/0.1 (research / non-commercial)",
        },
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def normalise_link(parent_page: str, raw_href: str) -> str | None:
    """Resolve `./folder/foo.htm` or `foo.htm` against the parent's
    directory. Returns a site-rooted path like `razgov/razgov01.htm`
    or `narech.htm` — i.e. what you'd append to ROOT + '/'.
    Returns None on malformed input or `index.htm` (loops back)."""
    href = raw_href.strip().lstrip("./")
    if not href or href.endswith("/") or "#" in href:
        return None
    # If parent has a folder, resolve relative refs against it.
    if "/" in parent_page:
        parent_dir = parent_page.rsplit("/", 1)[0]
        if not href.startswith(parent_dir + "/") and "/" not in href:
            href = f"{parent_dir}/{href}"
    return href


def crawl_pages(seed: str = "index.htm", max_pages: int = 1000) -> list[str]:
    """BFS over kaz-tili.kz subpages starting from `seed`. Returns
    discovered page paths (site-rooted). Skips JS / CSS / images;
    only follows .htm links inside the site."""
    seen: set[str] = {seed}
    queue: list[str] = [seed]
    out: list[str] = []
    while queue and len(seen) < max_pages:
        page = queue.pop(0)
        out.append(page)
        try:
            body = fetch(f"{ROOT}/{page}").decode("utf-8", errors="replace")
        except Exception as e:
            print(f"[warn] crawl fetch {page}: {e}", file=sys.stderr)
            continue
        for m in SUB_HREF.finditer(body):
            link = normalise_link(page, m.group(1))
            if link is None or link in seen:
                continue
            # Stay on-site (relative-path filter only — we never
            # build absolute URLs from these matches anyway).
            seen.add(link)
            queue.append(link)
        time.sleep(0.05)
    return out


def discover_subpages() -> list[str]:
    """Back-compat shim — full recursive crawl from index.htm."""
    return crawl_pages(seed="index.htm")


def extract_pairs(page_html: str) -> list[tuple[str, str]]:
    """
    Return [(audio_basename, kazakh_text)] pairs for one page.
    Strategy: find every <a href="./wav2/*.mp3">...</a> and read
    the chunk of HTML between this link's end-tag and the next
    audio link (or end of body). From that chunk, pull every
    <span class=kaz>...</span>, drop nested <span class=se>...
    </span> Russian glosses, strip remaining tags, decode HTML
    entities, and concatenate.
    """
    # Find all anchor positions to know where each segment ends.
    anchors = list(AUDIO_HREF.finditer(page_html))
    pairs = []
    for i, a in enumerate(anchors):
        basename = a.group(1).strip().split("/")[-1]
        end_anchor = a.end()
        # Segment ends at the next anchor's start (or document end).
        next_start = anchors[i + 1].start() if i + 1 < len(anchors) else len(page_html)
        segment = page_html[end_anchor:next_start]
        # Strip <span class=se>...</span> first (Russian gloss).
        segment_no_gloss = SPAN_SE.sub("", segment)
        # Collect text from every <span class=kaz>...</span> in segment.
        kaz_chunks = SPAN_KAZ.findall(segment_no_gloss)
        if not kaz_chunks:
            continue
        text = " ".join(kaz_chunks)
        # Strip remaining tags + decode entities.
        text = TAG.sub(" ", text)
        text = html.unescape(text)
        text = re.sub(r"\s+", " ", text).strip()
        if not text:
            continue
        # Quality gate: must contain at least one Kazakh-Cyrillic letter.
        if not any(c.lower() in KAZ_LETTERS for c in text):
            continue
        pairs.append((basename, text))
    return pairs


def fetch_subpage(page: str) -> str | None:
    try:
        return fetch(f"{ROOT}/{page}").decode("utf-8", errors="replace")
    except Exception as e:
        print(f"[warn] fetch {page}: {e}", file=sys.stderr)
        return None


def download_audio(basename: str, parent_page: str = "") -> bytes | None:
    """Try sibling-of-page wav/wav2 folders, then site-root /wav2/
    and /wav/ as fallback. Some sub-folders (razgov/, text/) host
    their own ./wav2/ directory, others share the root one."""
    # Candidate prefixes — relative-to-page and absolute-from-root.
    prefixes = []
    if "/" in parent_page:
        parent_dir = parent_page.rsplit("/", 1)[0]
        prefixes.extend([f"{parent_dir}/wav2", f"{parent_dir}/wav"])
    prefixes.extend(["wav2", "wav"])
    for prefix in prefixes:
        url = f"{ROOT}/{prefix}/{basename}"
        try:
            return fetch(url, timeout=30)
        except Exception:
            continue
    return None


def to_wav_16k_mono(mp3_bytes: bytes, dst: Path) -> bool:
    """Decode MP3 → 16 kHz mono WAV via ffmpeg."""
    try:
        r = subprocess.run(
            [
                "ffmpeg",
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-i",
                "pipe:0",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-c:a",
                "pcm_s16le",
                str(dst),
            ],
            input=mp3_bytes,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
        )
        return r.returncode == 0 and dst.exists() and dst.stat().st_size > 100
    except Exception as e:
        print(f"[warn] ffmpeg {dst.name}: {e}", file=sys.stderr)
        return False


def wav_duration_s(p: Path) -> float:
    # WAV header parse: bytes 24..28 = sample_rate, file size /
    # (sr * channels * bytes_per_sample) ≈ duration.
    try:
        with p.open("rb") as f:
            head = f.read(44)
        if len(head) < 44 or head[:4] != b"RIFF":
            return 0.0
        sr = int.from_bytes(head[24:28], "little")
        bits = int.from_bytes(head[34:36], "little")
        ch = int.from_bytes(head[22:24], "little")
        bps = max(1, (bits // 8) * ch)
        data_bytes = p.stat().st_size - 44
        return data_bytes / (sr * bps)
    except Exception:
        return 0.0


def main():
    AUDIO_DIR.mkdir(parents=True, exist_ok=True)
    # Resume support: skip basenames already in the manifest.
    seen_basenames: set[str] = set()
    if MANIFEST.exists():
        with MANIFEST.open() as f:
            for line in f:
                try:
                    d = json.loads(line)
                    seen_basenames.add(d.get("audio_basename", ""))
                except Exception:
                    continue
        print(f"[scrape] resume: {len(seen_basenames)} entries already in manifest")
    pages = discover_subpages()
    print(f"[scrape] discovered {len(pages)} subpages")

    out_f = MANIFEST.open("a", encoding="utf-8")
    total_pairs = 0
    total_downloaded = 0
    total_rejected = 0
    for page in pages:
        body = fetch_subpage(page)
        if body is None:
            continue
        pairs = extract_pairs(body)
        if not pairs:
            continue
        print(f"[scrape] {page}: {len(pairs)} audio/text pairs")
        for basename, text in pairs:
            total_pairs += 1
            if basename in seen_basenames:
                continue
            stem = basename.rsplit(".", 1)[0]
            label = f"kaztili_{stem}"
            wav_path = AUDIO_DIR / f"{label}.wav"
            if wav_path.exists():
                seen_basenames.add(basename)
                continue
            mp3 = download_audio(basename, parent_page=page)
            if mp3 is None or len(mp3) < 200:
                total_rejected += 1
                continue
            if not to_wav_16k_mono(mp3, wav_path):
                total_rejected += 1
                if wav_path.exists():
                    wav_path.unlink()
                continue
            dur = wav_duration_s(wav_path)
            if dur < 0.10:
                wav_path.unlink()
                total_rejected += 1
                continue
            entry = {
                "label": label,
                "audio_basename": basename,
                "audio_path": str(wav_path.relative_to(OUT.parent)),
                "transcript": text,
                "duration_s": round(dur, 4),
                "source_page": page,
                "source_url": f"{ROOT}/{page.rsplit('/', 1)[0]}/wav2/{basename}"
                if "/" in page
                else f"{ROOT}/wav2/{basename}",
            }
            out_f.write(json.dumps(entry, ensure_ascii=False) + "\n")
            out_f.flush()
            seen_basenames.add(basename)
            total_downloaded += 1
            if total_downloaded % 25 == 0:
                print(
                    f"[scrape] progress: {total_downloaded} downloaded, "
                    f"{total_rejected} rejected"
                )
            # Tiny politeness delay
            time.sleep(0.10)
        # Pause between pages
        time.sleep(0.20)
    out_f.close()
    print(f"[scrape] === done ===")
    print(f"[scrape] pairs found    : {total_pairs}")
    print(f"[scrape] downloaded     : {total_downloaded}")
    print(f"[scrape] rejected       : {total_rejected}")
    print(f"[scrape] manifest       : {MANIFEST}")


if __name__ == "__main__":
    main()
