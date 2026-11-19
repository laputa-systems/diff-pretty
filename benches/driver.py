#!/usr/bin/env python3
"""Small tiered runner for the checked-in rustybench workloads."""
import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
TIERS = ('edit', 'change', 'scenario', 'full')
CLI_BINARY = None
ROOT_NAMES = ('render_show_typical render_show_metadata render_show_large render_show_corpus '
              'render_document_show_010 render_reader_to_log_000 render_log_000 '
              'render_log_000_color render_plain_unified '
              'render_reader_document_synthetic_git_log_10000 pager_viewport_show_010').split()
SCRL_NAMES = ('viewport_redraw_no_search viewport_redraw_styled viewport_redraw_horizontal_clip '
              'session_redraw_without_movement initial_search_late_match initial_regex_search_late_match '
              'cached_search_redraw live_search_chunk_and_redraw filtered_stream_and_redraw_50k '
              'filtered_stream_and_redraw_500k filtered_stream_and_redraw_5m').split()
# Each existing operation remains available individually; the largest workload is manual.
WORK = [('self-test', 'edit', 'driver', 'selection and preflight contracts')]
WORK += [(n, 'scenario' if n == 'render_show_corpus' else 'change', 'render', 'render, stream or viewport operation') for n in ROOT_NAMES]
WORK += [('cli_render_show_010', 'change', 'cli', 'redirected render with byte oracle'),
         ('cli_pager_show_010', 'scenario', 'cli', 'terminal paging and search journey')]
WORK += [(n, 'full' if n.endswith('_5m') else 'change', 'pager', 'pager redraw, search or filtered stream') for n in SCRL_NAMES]


def select(args):
    names = {w[0] for w in WORK}
    areas = {w[2] for w in WORK}
    for values, valid, label in ((args.name, names, 'name'), (args.area, areas, 'area'), (args.skip, names, 'skip')):
        unknown = set(values) - valid
        if unknown:
            raise ValueError(f'unknown {label}: {", ".join(sorted(unknown))}')
    selected = [w for w in WORK if TIERS.index(w[1]) <= TIERS.index(args.tier)
                and (not args.name or w[0] in args.name)
                and (not args.area or w[2] in args.area) and w[0] not in args.skip]
    if not selected:
        raise ValueError('empty selection')
    return selected


def command(w):
    if w[0] == 'self-test':
        return [sys.executable, '-B', 'benches/test_driver.py']
    cmd = ['cargo', 'bench']
    if w[2] == 'pager':
        cmd += ['-p', 'scrl']
    options = [] if w[1] in ('scenario', 'full') else ['--sample-count', '1', '--sample-size', '1']
    return cmd + ['--bench', 'bench', '--', '--exact', 'bench::' + w[0]] + options


def preflight(selected, build_binary=True):
    global CLI_BINARY
    errors = []
    for tool in ({'cargo', 'rustc'} if any(w[2] != 'driver' for w in selected) else set()):
        if not shutil.which(tool):
            errors.append(f'{tool} missing; install the pinned Rust toolchain (needed by {", ".join(w[0] for w in selected if w[2] != "driver")})')
    if any((w[2] == 'render' and w[0] != 'render_reader_document_synthetic_git_log_10000') or w[0].startswith('filtered_stream_and_redraw_') for w in selected) and not list((ROOT / 'fixtures').glob('*.patch')):
        errors.append('fixtures/*.patch missing; restore checkout fixtures (needed by selected benchmarks)')
    if any(w[2] != 'driver' for w in selected) and shutil.disk_usage(ROOT).free < 100 * 1024 * 1024:
        errors.append('less than 100 MiB free; clear disk space for selected benchmarks')
    required = set()
    for name, _, area, _ in selected:
        if area != 'render':
            continue
        if name.startswith('render_show_'):
            required.update(('show_001 show_003 show_006 show_032 show_038 show_039 show_040 show_063 '
                             'show_078 show_082 show_095 show_098 show_015 show_041 show_048 show_059 '
                             'show_079 show_084 show_088 show_096 show_099').split())
        elif name in ('render_document_show_010', 'pager_viewport_show_010'):
            required.add('show_010')
        elif name in ('render_reader_to_log_000', 'render_log_000'):
            required.add('log_000')
        elif name == 'render_log_000_color':
            required.add('log_000_color')
        elif name == 'render_plain_unified':
            required.add('plain_unified')
    if any(w[2] == 'cli' for w in selected):
        required.add('show_010')
    missing = [name for name in sorted(required) if not (ROOT / 'fixtures' / (name + '.patch')).is_file()]
    if missing:
        errors.append('missing fixtures: ' + ', '.join(missing) + '; restore checkout fixtures')
    if build_binary and not errors and any(w[2] == 'cli' for w in selected):
        result = subprocess.run(['cargo', 'build', '--release', '--bin', 'diff-pretty', '--message-format=json'],
                                cwd=ROOT, capture_output=True, text=True)
        if result.returncode:
            errors.append('release diff-pretty build failed: ' + result.stderr)
        else:
            artifacts = [json.loads(line) for line in result.stdout.splitlines()]
            paths = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
                     and a.get('target', {}).get('name') == 'diff-pretty' and a.get('executable')]
            if len(paths) != 1 or not Path(paths[0]).is_file():
                errors.append('release diff-pretty executable missing from Cargo artifacts')
            else:
                CLI_BINARY = paths[0]
    return errors


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--tier', choices=TIERS, default='edit')
    p.add_argument('--name', action='append', default=[])
    p.add_argument('--area', action='append', default=[])
    p.add_argument('--skip', action='append', default=[])
    p.add_argument('--list', action='store_true')
    p.add_argument('--check', action='store_true')
    p.add_argument('--dry-run', action='store_true')
    args = p.parse_args()
    if args.list:
        for name, tier, area, purpose in WORK:
            print(f'{name}: tier={tier} area={area} estimated=minutes' if tier != 'edit' else f'{name}: tier={tier} area={area} estimated=seconds')
            print(f'  {purpose}; command: {shlex.join(command((name, tier, area, purpose)))}')
        return 0
    try:
        selected = select(args)
    except ValueError as e:
        print(e, file=sys.stderr)
        return 2
    for w in WORK:
        if w not in selected:
            print(f'excluded {w[0]}: selector or tier ceiling')
    errors = preflight(selected, build_binary=not args.dry_run)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f'preflight OK; host={platform.platform()} load={os.getloadavg()}')
    for w in selected:
        print(f'{w[0]}: {shlex.join(command(w))}')
    if args.check or args.dry_run:
        return 0
    directory = ROOT / '.work' / 'bench' / datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')
    directory.mkdir(parents=True)
    results = []
    for w in selected:
        start = time.monotonic()
        with (directory / (w[0] + '.log')).open('w') as log:
            env = os.environ.copy()
            if w[2] == 'cli':
                env['DIFF_PRETTY_BENCH_BINARY'] = CLI_BINARY
            result = subprocess.run(command(w), cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, env=env)
        status = result.returncode
        if w[2] != 'driver' and w[0] not in (directory / (w[0] + '.log')).read_text():
            print(f'{w[0]}: benchmark output missing; selection did not run', file=sys.stderr)
            status = status or 1
        results.append({'name': w[0], 'exit': status, 'seconds': round(time.monotonic() - start, 3)})
    summary = {'results': results, 'excluded': [w[0] for w in WORK if w not in selected],
               'exclusion_reason': 'selector or tier ceiling', 'host': platform.platform(),
               'timing_validity': 'not host-gated; no discarded samples reported; completion is not a performance verdict',
               'revision': subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True, text=True).stdout.strip(),
               'dirty': bool(subprocess.run(['git', 'status', '--porcelain'], cwd=ROOT, capture_output=True, text=True).stdout)}
    (directory / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(f'results: {directory}\n{json.dumps(summary, indent=2)}')
    return int(any(r['exit'] for r in results))


if __name__ == '__main__':
    sys.exit(main())
