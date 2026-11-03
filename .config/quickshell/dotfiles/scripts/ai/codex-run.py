#!/usr/bin/env python3
"""Run a Codex CLI turn and print the event stream the Pilot panel and bot read.

    codex-run.py --prompt TEXT [--session ID] [--model M] [--effort E]
                 [--sandbox read-only|workspace|full] [--cwd DIR]
                 [--max-wall-secs N] [--codex PATH]

The output is one JSON object per line, a stable shape both the panel and the bridge
parse:

    {"type":"progress","phase":"tool_started","call_id":..,"tool":"bash","label":..}
    {"type":"progress","phase":"tool_ran","call_id":..,"detail":..}
    {"type":"progress","phase":"tool_finished","call_id":..,"error":bool,"output":..}
    {"type":"progress","phase":"text","delta":..}
    {"type":"result","text":..,"session_id":..,"usage":{"output_tokens":..}}
    {"type":"error","message":..,"code":..}

Keeping it stable means the panel and the bridge never need a new parser when the
engine changes.
"""
import json
import os
import shutil
import subprocess
import sys
import threading
import time

IDEAL_MODEL_PATH = os.path.expanduser("~/.codex/models_cache.json")


def emit(row):
    print(json.dumps(row), flush=True)


def parse(argv):
    args = {
        "prompt": "",
        "session": "",
        "model": "",
        "effort": "",
        "sandbox": "workspace",
        "cwd": os.path.expanduser("~"),
        "max_wall_secs": 0,
        "codex": "",
    }
    i = 0
    while i < len(argv):
        key = argv[i]
        if key == "--prompt":
            args["prompt"] = argv[i + 1]
        elif key == "--session":
            args["session"] = argv[i + 1]
        elif key == "--model":
            args["model"] = argv[i + 1]
        elif key == "--effort":
            args["effort"] = argv[i + 1]
        elif key == "--sandbox":
            args["sandbox"] = argv[i + 1]
        elif key == "--cwd":
            args["cwd"] = argv[i + 1]
        elif key == "--max-wall-secs":
            args["max_wall_secs"] = int(float(argv[i + 1]))
        elif key == "--codex":
            args["codex"] = argv[i + 1]
        i += 2
    return args


def models():
    try:
        with open(IDEAL_MODEL_PATH) as handle:
            data = json.load(handle)
    except (OSError, ValueError):
        return []
    items = [m for m in data.get("models", []) if m.get("visibility") == "list"]
    items.sort(key=lambda m: m.get("priority", 0), reverse=True)
    return items


def default_model():
    items = models()
    return items[0]["slug"] if items else ""


def find_codex(given):
    for candidate in (
        given,
        os.environ.get("CODEX_BIN", ""),
        shutil.which("codex") or "",
        os.path.expanduser("~/.local/node/bin/codex"),
        "/usr/bin/codex",
    ):
        if candidate and os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return ""


def sandbox_flags(mode):
    # --approve-for-me and -s conflict, and resume has neither, so both modes
    # are expressed as config overrides.
    if mode == "full":
        return ["--dangerously-bypass-approvals-and-sandbox"]
    setting = "read-only" if mode == "read-only" else "workspace-write"
    return ["-c", f'sandbox_mode="{setting}"', "-c", 'approval_policy="never"']


def open_tool(item, kind):
    if kind == "command_execution":
        return "bash", item.get("command", "")
    if kind == "file_change":
        return "edit", describe_changes(item)
    if kind == "mcp_tool_call":
        return item.get("server", "mcp"), item.get("tool", "")
    if kind == "web_search":
        return "search", item.get("query", "")
    return kind, item.get("title", "") or item.get("text", "")[:80]


def describe_changes(item):
    parts = []
    for change in item.get("changes") or []:
        kind = change.get("kind", "update")
        parts.append(f"{kind} {change.get('path', '')}")
    return ", ".join(parts)


def main():
    args = parse(sys.argv[1:])
    if not args["prompt"]:
        emit({"type": "error", "message": "No prompt given.", "code": "usage"})
        return 1

    codex = find_codex(args["codex"])
    if not codex:
        emit({"type": "error", "message": "Codex CLI is not installed.", "code": "missing"})
        return 1

    model = args["model"] or default_model()
    command = [codex, "exec"]
    if args["session"]:
        command += ["resume", args["session"]]
    command += ["--json", "--skip-git-repo-check"]
    if model:
        command += ["-m", model]
    if args["effort"]:
        command += ["-c", f'model_reasoning_effort="{args["effort"]}"']
    command += sandbox_flags(args["sandbox"])
    command.append(args["prompt"])

    started = time.time()
    emit({"type": "progress", "phase": "generating"})

    try:
        child = subprocess.Popen(
            command,
            cwd=args["cwd"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
    except OSError as error:
        emit({"type": "error", "message": f"Could not start Codex: {error}", "code": "spawn"})
        return 1

    stderr_lines = []

    def drain():
        for line in child.stderr:
            line = line.strip()
            # Codex logs routine tool noise here; only the last lines are useful
            # and only when the turn fails.
            if line and "Reading additional input" not in line:
                stderr_lines.append(line)

    reader = threading.Thread(target=drain, daemon=True)
    reader.start()

    session_id = args["session"]
    pending_text = ""
    finished = False
    open_steps = set()

    def flush_text():
        nonlocal pending_text
        if pending_text:
            emit({"type": "progress", "phase": "text", "delta": pending_text + "\n\n"})
            pending_text = ""

    for line in child.stdout:
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue

        kind = row.get("type")
        if kind == "thread.started":
            session_id = row.get("thread_id", session_id)
            emit({"type": "progress", "phase": "generating", "session_id": session_id})
            continue

        if kind == "item.started":
            item = row.get("item") or {}
            tool, label = open_tool(item, item.get("type", "tool"))
            open_steps.add(item.get("id", ""))
            emit({
                "type": "progress",
                "phase": "tool_started",
                "call_id": item.get("id", ""),
                "tool": tool,
                "label": label,
                "session_id": session_id,
            })
            continue

        if kind == "item.completed":
            item = row.get("item") or {}
            item_type = item.get("type")
            call_id = item.get("id", "")
            if item_type == "agent_message":
                flush_text()
                pending_text = item.get("text", "")
            elif item_type == "command_execution":
                emit({
                    "type": "progress",
                    "phase": "tool_ran",
                    "call_id": call_id,
                    "detail": item.get("command", ""),
                    "session_id": session_id,
                })
                emit({
                    "type": "progress",
                    "phase": "tool_finished",
                    "call_id": call_id,
                    "error": item.get("exit_code") not in (0, None),
                    "output": item.get("aggregated_output", ""),
                    "session_id": session_id,
                })
                open_steps.discard(call_id)
            elif item_type == "file_change":
                emit({
                    "type": "progress",
                    "phase": "tool_finished",
                    "call_id": call_id,
                    "error": item.get("status") == "failed",
                    "output": describe_changes(item),
                    "session_id": session_id,
                })
                open_steps.discard(call_id)
            elif item_type == "error":
                emit({
                    "type": "error",
                    "message": item.get("message", "Codex reported an error."),
                    "code": "provider",
                    "session_id": session_id,
                })
            continue

        if kind == "turn.completed":
            # The last agent message is the answer; earlier ones were
            # already flushed as progress text.
            usage = row.get("usage") or {}
            emit({
                "type": "result",
                "text": pending_text,
                "session_id": session_id,
                "usage": {
                    "input_tokens": usage.get("input_tokens", 0),
                    "output_tokens": usage.get("output_tokens", 0)
                    + usage.get("reasoning_output_tokens", 0),
                },
                "duration_ms": int((time.time() - started) * 1000),
            })
            pending_text = ""
            finished = True
            continue

        if kind in ("turn.failed", "error"):
            message = (row.get("message") or (row.get("error") or {}).get("message")
                       or "Codex failed.")
            emit({
                "type": "error",
                "message": message,
                "code": kind,
                "session_id": session_id,
            })
            finished = True
    timed_out = False
    try:
        child.wait(timeout=max(1, args["max_wall_secs"]) if args["max_wall_secs"] else None)
    except subprocess.TimeoutExpired:
        child.kill()
        timed_out = True

    reader.join(timeout=1)
    if timed_out:
        emit({"type": "error", "message": "The task hit its time limit.", "code": "timeout"})
        return 1
    if not finished:
        detail = stderr_lines[-1] if stderr_lines else ""
        emit({
            "type": "error",
            "message": "Codex exited without answering." + (f"\n{detail}" if detail else ""),
            "code": "exit",
            "session_id": session_id,
        })
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
