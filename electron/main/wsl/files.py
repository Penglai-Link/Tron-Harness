"""File-panel operations run in Linux, scoped to a root selected by the user."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

LIMIT = 2 * 1024 * 1024


def native(value):
    real = Path(value).resolve(strict=True)
    filesystem = subprocess.check_output(["stat", "-f", "-c", "%T", str(real)], text=True).strip()
    if re.match(r"^/mnt/[a-z](?:/|$)", str(real), re.I) or filesystem in ("9p", "drvfs", "smb2"):
        raise ValueError("wsl-linux-path-required")
    return real


def target(root, relative, writing=False):
    if not isinstance(relative, str) or "\x00" in relative or "\\" in relative or len(relative) > 4096:
        raise ValueError("invalid-path")
    value = Path(relative)
    if value.is_absolute() or ".." in value.parts:
        raise ValueError("path-outside-root")
    candidate = root / value
    if candidate.exists() or candidate.is_symlink():
        resolved = native(candidate)
    elif writing:
        resolved = native(candidate.parent) / candidate.name
    else:
        raise ValueError("path-not-found")
    if not resolved.is_relative_to(root):
        raise ValueError("path-outside-root")
    if writing and resolved == root:
        raise ValueError("invalid-path")
    return resolved


def execute(request):
    root = native(request["root"])
    if not root.is_dir():
        raise ValueError("not-a-directory")
    command = request["command"]
    if command == "root":
        return {"path": str(root), "name": root.name or "/"}
    writing = command not in ("fs:list", "fs:read")
    item = target(root, request.get("path", ""), writing)
    if command == "fs:list":
        if not item.is_dir():
            raise ValueError("not-a-directory")
        entries = []
        for entry in item.iterdir():
            try:
                stat = entry.lstat()
                link = entry.is_symlink()
                entries.append({"name": entry.name, "isDir": not link and entry.is_dir(), "isFile": not link and entry.is_file(), "isSymlink": link, "size": stat.st_size if not link and entry.is_file() else 0})
            except OSError:
                continue
        return {"entries": sorted(entries, key=lambda entry: (not entry["isDir"], entry["name"]))}
    if command == "fs:read":
        if not item.is_file():
            raise ValueError("not-a-file")
        size = item.stat().st_size
        if size > LIMIT:
            return {"content": "", "binary": False, "truncated": True, "size": size}
        with item.open("rb") as source:
            data = source.read(LIMIT + 1)
        binary = b"\0" in data[:8192]
        return {"content": "" if binary else data[:LIMIT].decode("utf8", errors="replace"), "binary": binary, "truncated": len(data) > LIMIT, "size": size}
    if command == "fs:write":
        content = request.get("content")
        if not isinstance(content, str):
            raise ValueError("invalid-content")
        data = content.encode("utf8")
        if len(data) > LIMIT:
            raise ValueError("content-too-large")
        item.write_bytes(data)
    elif command == "fs:createFile":
        with item.open("x"):
            pass
    elif command == "fs:createDirectory":
        item.mkdir()
    elif command == "fs:rename":
        name = request.get("newName")
        if not isinstance(name, str) or not name or name in (".", "..") or any(char in name for char in "/\\\0") or len(name) > 255:
            raise ValueError("invalid-name")
        destination = item.with_name(name)
        if destination.exists() or destination.is_symlink():
            raise ValueError("already-exists")
        item.rename(destination)
    elif command == "fs:delete":
        if item.is_dir():
            if request.get("recursive"):
                shutil.rmtree(item)
            else:
                item.rmdir()
        else:
            item.unlink()
    else:
        raise ValueError("unknown-fs-command")
    return {}


try:
    os.umask(0o077)
    data = sys.stdin.buffer.read(8 * 1024 * 1024 + 1)
    if len(data) > 8 * 1024 * 1024:
        raise ValueError("content-too-large")
    print(json.dumps(execute(json.loads(data))))
except FileExistsError:
    print("already-exists", file=sys.stderr)
    sys.exit(1)
except FileNotFoundError:
    print("path-not-found", file=sys.stderr)
    sys.exit(1)
except Exception as error:
    print(str(error), file=sys.stderr)
    sys.exit(1)
