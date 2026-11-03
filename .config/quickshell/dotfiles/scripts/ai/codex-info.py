#!/usr/bin/env python3
"""Small queries about the local Codex install, for the Pilot panel.

    codex-info.py models             slug<TAB>label<TAB>efforts, best first
    codex-info.py sessions [N]       id<TAB>epoch<TAB>title<TAB>cwd, newest first
    codex-info.py history ID [N]     role: text, last N messages
    codex-info.py status             one line naming the installed Codex
"""
import json
import os
import shutil
import subprocess
import sys

CACHE = os.path.expanduser("~/.codex/models_cache.json")
SESSIONS = os.path.expanduser("~/.codex/sessions")


def models():
    try:
        with open(CACHE) as handle:
            data = json.load(handle)
    except (OSError, ValueError):
        return
    items = [m for m in data.get("models", []) if m.get("visibility") == "list"]
    items.sort(key=lambda m: m.get("priority", 0), reverse=True)
    for model in items:
        levels = [
            level.get("effort", level) if isinstance(level, dict) else level
            for level in (model.get("supported_reasoning_levels") or [])
        ]
        print("\t".join([model.get("slug", ""), model.get("display_name", ""), ",".join(levels)]))


def sessions(limit):
    rows = []
    for root, _, files in os.walk(SESSIONS):
        for name in files:
            if not name.endswith(".jsonl"):
                continue
            path = os.path.join(root, name)
            try:
                rows.append((os.path.getmtime(path), path))
            except OSError:
                continue
    rows.sort(reverse=True)

    printed = 0
    for stamp, path in rows:
        session_id, title, cwd = "", "", ""
        try:
            with open(path, errors="replace") as handle:
                for line in handle:
                    try:
                        entry = json.loads(line)
                    except ValueError:
                        continue
                    payload = entry.get("payload") or {}
                    if entry.get("type") == "session_meta":
                        session_id = payload.get("session_id", "")
                        cwd = payload.get("cwd", "")
                        continue
                    if entry.get("type") != "response_item":
                        continue
                    if payload.get("type") != "message" or payload.get("role") != "user":
                        continue
                    text = " ".join(
                        part.get("text", "")
                        for part in (payload.get("content") or [])
                        if isinstance(part, dict)
                    )
                    if text.startswith("# AGENTS.md") or not text.strip():
                        continue
                    title = " ".join(text.split())[:60]
                    break
        except OSError:
            continue
        if not session_id:
            continue
        print("\t".join([session_id, str(int(stamp)), title or "untitled", cwd or ""]))
        printed += 1
        if printed >= limit:
            return


def rollout(session_id):
    for root, _, files in os.walk(SESSIONS):
        for name in files:
            if name.endswith(f"-{session_id}.jsonl"):
                return os.path.join(root, name)
    return ""


def history(session_id, limit):
    path = rollout(session_id)
    if not path:
        print(f"No session {session_id}.")
        return 1
    messages = []
    with open(path, errors="replace") as handle:
        for line in handle:
            try:
                entry = json.loads(line)
            except ValueError:
                continue
            if entry.get("type") != "response_item":
                continue
            payload = entry.get("payload") or {}
            if payload.get("type") != "message":
                continue
            role = payload.get("role", "")
            if role not in ("user", "assistant"):
                continue
            text = " ".join(
                part.get("text", "")
                for part in (payload.get("content") or [])
                if isinstance(part, dict)
            ).strip()
            if not text or text.startswith("# AGENTS.md"):
                continue
            messages.append((role, " ".join(text.split())))
    if not messages:
        print(f"Session {session_id} has no messages.")
        return 0
    for role, text in messages[-limit:]:
        label = "You" if role == "user" else "Pilot"
        print(f"{label}: {text[:400]}")
    return 0


def status():
    for candidate in (
        os.environ.get("CODEX_BIN", ""),
        shutil.which("codex") or "",
        os.path.expanduser("~/.local/node/bin/codex"),
        "/usr/bin/codex",
    ):
        if candidate and os.path.isfile(candidate):
            try:
                out = subprocess.run(
                    [candidate, "--version"], capture_output=True, text=True, timeout=20
                )
                version = (out.stdout or out.stderr).strip().splitlines()[0]
                print(f"{version or 'codex'}")
                return 0
            except (OSError, subprocess.SubprocessError, IndexError):
                continue
    print("codex not found")
    return 1


def main():
    what = sys.argv[1] if len(sys.argv) > 1 else ""
    if what == "models":
        models()
    elif what == "sessions":
        sessions(int(sys.argv[2]) if len(sys.argv) > 2 else 12)
    elif what == "history":
        if len(sys.argv) < 3:
            print("history needs a session id", file=sys.stderr)
            return 2
        return history(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 10)
    elif what == "status":
        return status()
    else:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
