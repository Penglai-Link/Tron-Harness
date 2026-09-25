#!/usr/bin/env python3
"""Create, validate and deterministically package Tron skins. Python stdlib only."""
import argparse
import io
import json
import re
import stat
import sys
import zipfile
from pathlib import Path
from urllib.parse import urlsplit

SKILL = Path(__file__).resolve().parents[1]
MAX_PACKAGE = 20 * 1024 * 1024
MAX_EXPANDED = 40 * 1024 * 1024
MAX_MANIFEST = 256 * 1024
IMAGE = {'.png', '.jpg', '.jpeg', '.webp', '.gif'}
FONT = {'.woff', '.woff2', '.ttf', '.otf'}
CSS = {'.css'}
VISUAL_CSS = set('color background background-color background-image background-position background-size background-repeat background-blend-mode border border-color border-style border-width border-radius outline outline-color outline-style outline-width box-shadow text-shadow opacity filter backdrop-filter mix-blend-mode animation animation-name animation-duration animation-delay animation-direction animation-fill-mode animation-iteration-count animation-play-state animation-timing-function transition transition-property transition-duration transition-delay transition-timing-function transform content font-family font-style font-weight text-decoration text-decoration-color text-decoration-style text-decoration-line -webkit-text-stroke -webkit-text-fill-color'.split())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def safe_path(name):
    return isinstance(name, str) and 0 < len(name.encode()) <= 240 and all(
        re.fullmatch(r'[A-Za-z0-9_.-]+', part) and part not in ('.', '..')
        for part in name.split('/'))


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f'Duplicate JSON key: {key}')
        result[key] = value
    return result


def manifest(raw):
    require(len(raw) <= MAX_MANIFEST, 'skin.json exceeds 256 KiB')
    data = json.loads(raw, object_pairs_hook=unique_object)
    require(isinstance(data, dict), 'Manifest must be an object')
    require(not set(data) - {'id', 'name', 'version', 'mode', 'colors', 'layout', 'resources'}, 'Unknown manifest field')
    require(isinstance(data.get('id'), str) and re.fullmatch(r'[a-z0-9-]{1,64}', data['id']), 'Invalid id')
    require(data['id'] not in ('light', 'matrix-dark'), 'Use a new id, not a built-in id')
    require(isinstance(data.get('name'), str) and data['name'].strip() and len(data['name'].encode()) <= 120, 'Invalid name')
    require(type(data.get('version')) is int and data['version'] in (1, 2), 'version must be 1 or 2')
    require(data.get('mode') in ('Light', 'Dark'), 'mode must be Light or Dark')
    variables = {}
    count = 0
    for field in ('colors', 'layout'):
        values = data.get(field, {})
        require(isinstance(values, dict), f'{field} must be an object')
        count += len(values)
        for key, value in values.items():
            require(re.fullmatch(r'--[a-z0-9-]+', key) and len(key) <= 100 and key != '--tron-system-font-size', f'Invalid/reserved variable: {key}')
            require(isinstance(value, str) and 0 < len(value.encode()) <= 512, f'Invalid value for {key}')
            require(not re.search(r'[;{}<>\\@]|url\s*\(|/\*|!important', value, re.I), f'Forbidden CSS in {key}')
        variables.update(values)
    require(count <= 512, 'At most 512 variables across colors and layout')
    known_file = SKILL / 'references' / 'tokens.json'
    known = set(json.loads(known_file.read_text())) if known_file.exists() else set()
    for key in variables:
        if known and key not in known:
            print(f'WARNING: undocumented variable {key}; confirm a consumer exists', file=sys.stderr)
    # Reject direct/indirect cycles in author-supplied variables; CSS silently unsets them.
    visited = set()
    def visit(key, stack):
        require(key not in stack, f'CSS variable cycle: {" -> ".join([*stack, key])}')
        if key in visited:
            return
        for target in re.findall(r'var\(\s*(--[a-z0-9-]+)', variables[key]):
            if target in variables:
                visit(target, [*stack, key])
        visited.add(key)
    for key in variables:
        visit(key, [])
    resources = data.get('resources', {})
    require(isinstance(resources, dict) and not set(resources) - {'background', 'font', 'css'}, 'Invalid resources object')
    require(not resources.get('css') or data['version'] == 2, 'CSS resources require format version 2')
    local = []
    for kind, value in resources.items():
        if value is None:
            continue
        require(isinstance(value, str), f'Invalid {kind} resource')
        if value.startswith('https://'):
            require(kind != 'css', 'CSS must be stored inside the theme package')
            url = urlsplit(value)
            require(url.hostname and not url.username and url.password is None and len(value.encode()) <= 2048 and not re.search(r'["\'\\\n\r<>]', value), 'Invalid HTTPS resource URL')
            print(f'WARNING: {kind} needs network access; fonts also need CORS', file=sys.stderr)
        else:
            require(safe_path(value), f'Unsafe resource path: {value}')
            require(Path(value).suffix in (FONT if kind == 'font' else CSS if kind == 'css' else IMAGE), f'Wrong resource type: {value}')
            local.append(value)
    return data, local


def read_file(path, limit):
    require(path.is_file() and not path.is_symlink(), f'Missing file or symlink: {path}')
    require(path.stat().st_size <= limit, f'File exceeds limit: {path.name}')
    return path.read_bytes()


def css_resources(raw, filename):
    try:
        source = raw.decode('utf-8')
    except UnicodeDecodeError as error:
        raise ValueError(f'{filename}: CSS must be UTF-8') from error
    require(len(raw) <= 1024 * 1024, f'{filename}: CSS exceeds 1 MiB')
    require('\\' not in source, f'{filename}: CSS escapes are not supported')
    require(not re.search(r'@\s*(?!keyframes\b|-webkit-keyframes\b)[\w-]+|!important|expression\s*\(|javascript:', source, re.I), f'{filename}: forbidden at-rule or unsafe CSS')
    refs = []
    for value in re.findall(r"""url\(\s*['"]?([^)'"]+)['"]?\s*\)""", source, re.I):
        require(safe_path(value), f'{filename}: unsafe url() resource: {value}')
        refs.append(value)
    # The backend performs structural CSS parsing too. This authoring check catches
    # geometry changes early and reports the source declaration before packaging.
    clean = re.sub(r"""/\*.*?\*/|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'""", '', source, flags=re.S)
    overlay_geometry = set('position inset inset-block inset-inline top right bottom left width height min-width min-height max-width max-height'.split())
    for match in re.finditer(r'(?:^|[;{])\s*([\w-]+)\s*:', clean, re.M):
        prop = match.group(1).lower()
        opening = clean.rfind('{', 0, match.start())
        closing = clean.rfind('}', 0, opening)
        selector = clean[closing + 1:opening].strip()
        overlay = bool(re.fullmatch(r'\[data-theme-effect-layer(?:=[^\]]+)?\](?:::before|::after)?', selector, re.I))
        require(prop.startswith('--') or prop in VISUAL_CSS or (overlay and prop in overlay_geometry), f'{filename}: property {prop} can change component geometry')
    return refs


def load(source):
    """Return manifest and bounded, validated package members; never extract ZIPs."""
    if source.is_dir():
        require(not source.is_symlink(), 'Symlink package directory is not allowed')
        raw = read_file(source / 'skin.json', MAX_MANIFEST)
        data, refs = manifest(raw)
        files = {'skin.json': raw}
        total = len(raw)
        pending = list(dict.fromkeys(refs))
        while pending:
            name = pending.pop()
            path = source / name
            require(not any((source / Path(*Path(name).parts[:i])).is_symlink() for i in range(1, len(Path(name).parts) + 1)), f'Symlink resource: {name}')
            require(path.resolve().is_relative_to(source.resolve()), f'Resource escapes package: {name}')
            require(path.is_file(), f'Missing resource: {name}')
            total += path.stat().st_size
            require(total <= MAX_EXPANDED, 'Expanded package exceeds 40 MiB')
            files[name] = read_file(path, MAX_EXPANDED)
            if Path(name).suffix == '.css':
                pending.extend(ref for ref in css_resources(files[name], name) if ref not in files and ref not in pending)
    else:
        raw = read_file(source, MAX_PACKAGE)
        if raw.startswith(b'PK'):
            files = {}
            with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                entries = archive.infolist()
                require(len(entries) <= 128, 'ZIP has more than 128 entries')
                require(sum(entry.file_size for entry in entries) <= MAX_EXPANDED, 'Expanded package exceeds 40 MiB')
                names = set()
                for entry in entries:
                    name = entry.filename.rstrip('/')
                    require(safe_path(name) and name not in names, f'Unsafe/duplicate ZIP entry: {name}')
                    names.add(name)
                    require(not stat.S_ISLNK(entry.external_attr >> 16), f'Symlink ZIP entry: {name}')
                    if entry.is_dir():
                        continue
                    require(name == 'skin.json' or Path(name).suffix in IMAGE | FONT | CSS, f'Unsupported file: {name}')
                    with archive.open(entry) as stream:
                        content = stream.read(min(entry.file_size, MAX_EXPANDED) + 1)
                    require(len(content) == entry.file_size, f'Invalid size: {name}')
                    files[name] = content
            require('skin.json' in files, 'ZIP root must contain skin.json')
            data, refs = manifest(files['skin.json'])
            for name, content in files.items():
                if Path(name).suffix == '.css':
                    refs.extend(css_resources(content, name))
        else:
            data, refs = manifest(raw)
            require(not refs, 'Standalone JSON cannot reference local resources; use a ZIP')
            files = {'skin.json': raw}
        for name in refs:
            require(name in files, f'Missing resource: {name}')
    require(len(files) <= 128, 'Too many package files')
    return data, files


def pack(source, destination):
    data, files = load(source)
    require(not destination.exists(), f'Output already exists: {destination}')
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            archive.writestr(info, content)
    require(buffer.tell() <= MAX_PACKAGE, 'ZIP exceeds 20 MiB')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open('xb') as output:
        output.write(buffer.getvalue())
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    init = commands.add_parser('init')
    init.add_argument('directory', type=Path)
    init.add_argument('--id', required=True)
    init.add_argument('--name', required=True)
    init.add_argument('--mode', choices=['Light', 'Dark'], default='Light')
    validate = commands.add_parser('validate')
    validate.add_argument('source', type=Path)
    package = commands.add_parser('pack')
    package.add_argument('source', type=Path)
    package.add_argument('destination', type=Path)
    args = parser.parse_args()
    try:
        if args.command == 'init':
            require(not args.directory.exists(), 'Destination already exists')
            data = json.loads((SKILL / 'assets' / args.mode.lower() / 'skin.json').read_text())
            data.update(id=args.id, name=args.name)
            raw = (json.dumps(data, ensure_ascii=False, indent=2) + '\n').encode()
            manifest(raw)
            args.directory.mkdir(parents=True)
            (args.directory / 'skin.json').write_bytes(raw)
        elif args.command == 'validate':
            data, _ = load(args.source)
        else:
            data = pack(args.source, args.destination)
        print(f'OK: {args.command} {data["id"]}')
    except (ValueError, OSError, zipfile.BadZipFile, RuntimeError, RecursionError) as error:
        parser.exit(1, f'ERROR: {error}\n')


if __name__ == '__main__':
    main()
