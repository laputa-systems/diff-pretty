# Adapted from srv/scripts/mutants_gate.py.
# Copyright 2019 JoshuaRLi (Joshua Li)
#
# Permission to use, copy, modify, and/or distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
# IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
import argparse
import contextlib
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / '.cargo/mutants-gate.toml'
WORK = ROOT / '.work/mutants-gate'
GATE = {'Makefile', '.github/workflows/ci.yml', '.github/tools.lock.json', '.cargo/mutants.toml',
        '.cargo/mutants-gate.toml', 'scripts/mutants_gate.py', 'scripts/test_mutants_gate.py'}
SHARDS = ('cli', 'config', 'render', 'source', 'session', 'other')


class GateError(Exception):
    pass


def classify(path, status, config):
    if path in GATE:
        return 'gate'
    if any(fnmatch.fnmatchcase(path, pattern) for pattern in config['irrelevant']):
        return 'irrelevant'
    if (path.startswith('src/') or path.startswith('scrl/src/')) and path.endswith('.rs'):
        return 'deleted production' if status == 'D' else 'production'
    if path.startswith(('tests/', 'scrl/tests/', 'fixtures/', 'benches/', 'scrl/benches/')):
        return 'tests or fixtures'
    if path.endswith('.pyc') or path in {'.gitignore', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain', 'rust-toolchain.toml', 'build.rs'} or path.startswith('.cargo/') or Path(path).name in {'Cargo.toml', 'Cargo.lock', 'build.rs'}:
        return 'build configuration'
    return 'unclassified'


def parse_changes(data):
    fields = data.split('\0')
    if not fields or fields[-1] != '':
        raise GateError('malformed git change list')
    result = {}
    i = 0
    try:
        while i < len(fields) - 1:
            status = fields[i]
            i += 1
            if not status or status[0] not in 'ACDMRT':
                raise ValueError('unknown status')
            if status[0] in 'RC':
                old, new = fields[i:i + 2]
                i += 2
                result[old] = 'D' if status[0] == 'R' else 'M'
                result[new] = 'A'
            else:
                result[fields[i]] = status[0]
                i += 1
        if i != len(fields) - 1 or '' in result:
            raise ValueError('truncated record')
    except (ValueError, IndexError) as error:
        raise GateError(f'malformed git change list: {error}') from error
    return result


def test_boundary(text):
    """Return a terminal test module start only when its end is provable."""
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        if not line.strip().startswith(('#[cfg(test)]', '#[cfg(all(test,')):
            continue
        offset = sum(map(len, lines[:i + 1]))
        match = re.match(r'\s*(?:pub\s+)?mod\s+tests\s*\{', text[offset:])
        if not match:
            continue
        start = offset + match.end() - 1
        cursor, depth = start, 0
        while cursor < len(text):
            if text.startswith('//', cursor):
                end = text.find('\n', cursor + 2)
                cursor = len(text) if end < 0 else end
            elif text.startswith('/*', cursor):
                cursor += 2
                comments = 1
                while cursor < len(text) and comments:
                    if text.startswith('/*', cursor):
                        comments += 1
                        cursor += 2
                    elif text.startswith('*/', cursor):
                        comments -= 1
                        cursor += 2
                    else:
                        cursor += 1
                if comments:
                    return None
            else:
                raw = re.match(r'r(#{0,255})"', text[cursor:])
                if raw:
                    end = text.find('"' + raw[1], cursor + len(raw[0]))
                    if end < 0:
                        return None
                    cursor = end + 1 + len(raw[1])
                elif text[cursor] == '"':
                    cursor += 1
                    while cursor < len(text) and text[cursor] != '"':
                        cursor += 2 if text[cursor] == '\\' else 1
                    if cursor >= len(text):
                        return None
                    cursor += 1
                elif text[cursor] == "'" and (char := re.match(r"'(?:\\u\{[0-9a-fA-F_]+\}|\\.|[^'\\])'", text[cursor:])):
                    cursor += len(char[0])
                elif text[cursor] == '{':
                    depth += 1
                    cursor += 1
                elif text[cursor] == '}':
                    depth -= 1
                    cursor += 1
                    if depth == 0:
                        return i + 1 if not text[cursor:].strip() else None
                else:
                    cursor += 1
        return None
    return None


def regions(diff, old, new):
    """Classify changed nonblank lines on each side of the test boundary."""
    old_boundary, new_boundary = test_boundary(old), test_boundary(new)
    marker = re.compile(r'#\s*\[\s*cfg\s*\([^]]*\btest\b', re.S)
    uncertain = any(marker.search(text) and boundary is None for text, boundary in ((old, old_boundary), (new, new_boundary)))
    outside_tests = any(boundary is not None and marker.search(''.join(text.splitlines(keepends=True)[:boundary - 1])) for text, boundary in ((old, old_boundary), (new, new_boundary)))
    production, tests = uncertain, uncertain or outside_tests
    old_line = new_line = None
    hunks = 0
    for line in diff.splitlines():
        match = re.match(r'^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@', line)
        if match:
            old_line, new_line = int(match[1]), int(match[2])
            hunks += 1
            continue
        if old_line is None:
            continue
        if line.startswith('\\ No newline'):
            continue
        if line.startswith('-'):
            if line[1:].strip():
                production |= old_boundary is None or old_line < old_boundary
                tests |= old_boundary is not None and old_line >= old_boundary
            old_line += 1
        elif line.startswith('+'):
            if line[1:].strip():
                production |= new_boundary is None or new_line < new_boundary
                tests |= new_boundary is not None and new_line >= new_boundary
            new_line += 1
        elif line.startswith(' '):
            old_line += 1
            new_line += 1
        else:
            raise GateError('malformed diff hunk')
    if diff and not hunks:
        raise GateError('cannot classify diff hunks')
    return production, tests


def added_lines(diff):
    result = set()
    line_no = None
    for line in diff.splitlines():
        match = re.match(r'^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@', line)
        if match:
            line_no = int(match[1])
        elif line_no is None or line.startswith('\\ No newline'):
            continue
        elif line.startswith('+'):
            result.add(line_no)
            line_no += 1
        elif line.startswith(' '):
            line_no += 1
        elif not line.startswith('-'):
            raise GateError('malformed diff hunk')
    return result


def overlaps_changed_lines(entries, path, lines):
    try:
        for item in entries:
            if item['file'] != path:
                continue
            start = item['span']['start']['line']
            end = item['span']['end']['line']
            if not isinstance(start, int) or not isinstance(end, int) or start < 1 or end < start:
                raise ValueError('invalid span')
            if any(start <= line <= end for line in lines):
                return True
        return False
    except (KeyError, TypeError, ValueError) as error:
        raise GateError(f'invalid mutant span: {error}') from error


def shard_for(name):
    path = name.split(':', 1)[0]
    return {'src/main.rs': 'cli', 'scrl/src/main.rs': 'cli',
            'src/config.rs': 'config', 'src/render.rs': 'render',
            'src/source.rs': 'source', 'scrl/src/source.rs': 'source',
            'scrl/src/session.rs': 'session'}.get(path, 'other')


def parse_listing(text):
    if not text.strip():
        return []
    try:
        entries = json.loads(text)
        if not isinstance(entries, list):
            raise ValueError('listing is not an array')
        names = [entry['name'] for entry in entries]
        if any(not isinstance(name, str) or not name for name in names) or len(names) != len(set(names)):
            raise ValueError('invalid or duplicate identity')
        return names
    except (ValueError, KeyError, TypeError) as error:
        raise GateError(f'selection error: {error}') from error


def read_results(path, selected, baseline):
    try:
        data = json.loads(path.read_text())
        outcomes = data['outcomes']
        if not isinstance(outcomes, list):
            raise ValueError('outcomes is not a list')
        results = {}
        bases = []
        summaries = {'CaughtMutant': 'caught', 'MissedMutant': 'missed', 'Timeout': 'timeout', 'Unviable': 'unviable'}
        for outcome in outcomes:
            scenario = outcome['scenario']
            if scenario == 'Baseline':
                bases.append(outcome['summary'])
                continue
            name = scenario['Mutant']['name']
            summary = summaries[outcome['summary']]
            if name in results:
                raise ValueError('duplicate mutant')
            results[name] = summary
        if (baseline and bases != ['Success']) or (not baseline and bases):
            raise ValueError('missing, failed or unexpected baseline')
        if set(results) != set(selected) or data['total_mutants'] != len(selected):
            raise ValueError('tested identities differ from selection')
        for kind in summaries.values():
            if data[kind] != list(results.values()).count(kind):
                raise ValueError('summary count mismatch')
        return results
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise GateError(f'incomplete result: {path}: {error}') from error


def verdict(results):
    for kind in ('missed', 'timeout'):
        names = [name for name, result in results.items() if result == kind]
        if names:
            raise GateError(f'{kind}: ' + ', '.join(names))


def disposition(results, config, root):
    for item in config.get('equivalent', []):
        name = item['name']
        if results.get(name) != 'missed':
            continue
        if not item.get('reason') or not name.startswith(item['file'] + ':'):
            raise GateError(f'invalid equivalence disposition: {name}')
        if hashlib.sha256((root / item['file']).read_bytes()).hexdigest() == item['sha256']:
            results[name] = 'equivalent'
    return results


def exact_regex(names):
    return '^(?:' + '|'.join(re.sub(r'([\\.^$|?*+()\[\]{}])', r'\\\1', name) for name in names) + ')$'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('base')
    parser.add_argument('--shard', choices=SHARDS, required=True)
    opts = parser.parse_args()
    if CONFIG.is_symlink():
        raise GateError('symlink gate configuration is unsupported')
    config = tomllib.loads(CONFIG.read_text())
    deadline = time.monotonic() + config['budget_seconds']
    started = time.monotonic()
    WORK.mkdir(parents=True, exist_ok=True)
    common = ['cargo', 'mutants', '--no-config', '--gitignore', 'true', '--workspace', '--skip-calls-defaults=false']
    pending = []

    def run(args, allowed=(0,), capture=True):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise GateError(f'budget exhausted; {len(pending)} unresolved: {pending}')
        p = subprocess.Popen(args, cwd=ROOT, stdout=subprocess.PIPE if capture else None,
                             stderr=subprocess.PIPE if capture else None, text=True, start_new_session=True)
        try:
            out, err = p.communicate(timeout=remaining)
        except subprocess.TimeoutExpired as error:
            os.killpg(p.pid, signal.SIGKILL)
            p.communicate()
            raise GateError(f'budget exhausted; {len(pending)} unresolved: {pending}') from error
        except BaseException:
            os.killpg(p.pid, signal.SIGKILL)
            p.communicate()
            raise
        if p.returncode not in allowed:
            raise GateError(f'command failed ({p.returncode}): {args}: {err or ""}')
        return out if capture else ''

    def listing(args, details=False):
        text = run(common + ['--list', '--json'] + args)
        names = parse_listing(text)
        return (names, json.loads(text) if text.strip() else []) if details else names

    with contextlib.nullcontext(Path(tempfile.mkdtemp(prefix='run-', dir=WORK))) as scratch:
        print(f'receipt: {scratch}', flush=True)
        base = run(['git', 'merge-base', opts.base, 'HEAD']).strip()
        head = run(['git', 'rev-parse', 'HEAD']).strip()
        changes = parse_changes(run(['git', 'diff', '--name-status', '-M', '-z', base, '--']))
        untracked = run(['git', 'ls-files', '--others', '--exclude-standard', '-z'])
        for path in untracked.split('\0'):
            if path:
                changes[path] = 'A'
        classes = {path: classify(path, status, config) for path, status in changes.items()}
        for path, kind in sorted(classes.items()):
            print(f'{kind}: {path} ({changes[path]})', flush=True)
        if 'unclassified' in classes.values():
            raise GateError('unclassified change')
        for path, status in changes.items():
            if status != 'D' and (ROOT / path).is_symlink():
                raise GateError(f'symlink change is unsupported: {path}')
        selected = set()
        critical = any(kind in {'gate', 'build configuration', 'tests or fixtures', 'deleted production'} for kind in classes.values())
        for index, (path, kind) in enumerate(sorted(classes.items())):
            if kind != 'production':
                continue
            if any(c in path for c in ':\n*?[]'):
                raise GateError(f'unsupported production path: {path!r}')
            diff = run(['git', 'diff', '--no-ext-diff', '--unified=0', base, '--', path])
            if changes[path] == 'A' and not diff:
                diff = run(['git', 'diff', '--no-index', '--unified=0', '--', '/dev/null', path], allowed=(0, 1))
            if not diff:
                raise GateError(f'missing diff for {path}')
            old = '' if changes[path] == 'A' else run(['git', 'show', f'{base}:{path}'])
            current = (ROOT / path).read_text()
            prod, tests = regions(diff, old, current)
            critical |= tests
            print(f'regions: {path}: production={prod} tests={tests}', flush=True)
            if prod:
                file_diff = scratch / f'{index}.diff'
                file_diff.write_text(diff)
                names, entries = listing(['--in-diff', str(file_diff)], details=True)
                if not overlaps_changed_lines(entries, path, added_lines(diff)):
                    full = listing(['--file', path])
                    if not full:
                        raise GateError(f'production file has no mutants: {path}')
                    names += full
                    print(f'fallback whole file: {path}', flush=True)
                selected.update(names)
        if critical:
            for item in config['critical']:
                args = ['--file', item['file']]
                if 'regex' in item:
                    args += ['--re', item['regex']]
                names = listing(args)
                if not names:
                    raise GateError(f'empty critical component: {item}')
                print(f'critical: {item["file"]} {item.get("regex", "*")} = {len(names)}', flush=True)
                selected.update(names)
        relevant = any(kind != 'irrelevant' for kind in classes.values())
        if relevant and not selected:
            raise GateError('unexpected empty relevant selection')
        if not relevant:
            print('irrelevant change: no mutants selected', flush=True)
            return
        groups = {shard: sorted(n for n in selected if shard_for(n) == shard) for shard in SHARDS}
        if set().union(*(set(v) for v in groups.values())) != selected or sum(map(len, groups.values())) != len(selected):
            raise GateError('shard partition does not match selection')
        print(f'base={base} head={head} config_sha256={hashlib.sha256(CONFIG.read_bytes()).hexdigest()} shard={opts.shard}', flush=True)
        print(run(['rustc', '-vV']).strip(), flush=True)
        print(run(common + ['--version']).strip(), flush=True)
        for shard, names in groups.items():
            print(f'partition {shard}: {len(names)}', flush=True)
            for name in names:
                print(f'  {name}', flush=True)
        pending[:] = groups[opts.shard]
        if not pending:
            print('empty shard, full selection accounted for', flush=True)
            return
        files = sorted({name.split(':', 1)[0] for name in pending})
        args = sum((['--file', path] for path in files), []) + ['--re', exact_regex(pending)]
        actual = set(listing(args))
        if actual != set(pending):
            raise GateError(f'shard selection does not match inventory; missing={sorted(set(pending) - actual)} extra={sorted(actual - set(pending))}')
        output = scratch / 'result'
        caller = ['--cargo-test-arg=--test', '--cargo-test-arg=cli'] if opts.shard == 'render' else []
        run(common + caller + ['--baseline', 'run', '--timeout', str(config['timeout_seconds']),
                      '--build-timeout', str(config['build_timeout_seconds']), '--jobs', '1',
                      '--test-workspace', 'true', '--output', str(output)] + args,
            allowed=(0, 2, 3), capture=False)
        results = read_results(output / 'mutants.out/outcomes.json', pending, True)
        pending[:] = [n for n, result in results.items() if result == 'timeout']
        for index, name in enumerate(list(pending)):
            retry = ['--file', name.split(':', 1)[0], '--re', exact_regex([name])]
            if listing(retry) != [name]:
                raise GateError(f'timeout selection mismatch: {name}')
            out = scratch / f'retry-{index}'
            run(common + caller + ['--baseline', 'run', '--timeout', str(config['timeout_seconds'] * 3),
                          '--build-timeout', str(config['build_timeout_seconds']), '--jobs', '1',
                          '--test-workspace', 'true', '--output', str(out)] + retry,
                allowed=(0, 2, 3), capture=False)
            again = read_results(out / 'mutants.out/outcomes.json', [name], True)[name]
            if again == 'timeout':
                known = next((h for h in config.get('confirmed_hangs', []) if h['name'] == name), None)
                if known and hashlib.sha256((ROOT / known['file']).read_bytes()).hexdigest() == known['sha256']:
                    results[name] = 'confirmed hang'
                else:
                    results[name] = 'timeout'
            else:
                results[name] = again
            pending.remove(name)
        disposition(results, config, ROOT)
        for name, result in results.items():
            print(f'{result}: {name}', flush=True)
        print(f'wall_seconds={time.monotonic() - started:.1f}', flush=True)
        verdict(results)


if __name__ == '__main__':
    def interrupted(signum, frame):
        raise KeyboardInterrupt(f'signal {signum}')

    for signum in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    try:
        main()
    except (GateError, OSError, ValueError, UnicodeError, KeyError, KeyboardInterrupt) as error:
        print(f'gate failure: {error}', file=sys.stderr)
        sys.exit(1)
