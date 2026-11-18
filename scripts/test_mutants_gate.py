# Adapted from srv/scripts/test_mutants_gate.py.
# Copyright 2019 JoshuaRLi (Joshua Li)
# Permission to use, copy, modify, and/or distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR
# IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
import json
from pathlib import Path
import tempfile
import unittest

from mutants_gate import GateError, SHARDS, added_lines, classify, disposition, exact_regex, overlaps_changed_lines, parse_changes, parse_listing, read_results, regions, shard_for, verdict

CONFIG = {'irrelevant': ['*.md', 'docs/**', 'LICENSE']}


class SelectionTests(unittest.TestCase):
    def test_all_change_classes_and_rename(self):
        changes = parse_changes('D\0tests/cli.rs\0R100\0src/old.rs\0scrl/src/new.rs\0M\0Cargo.lock\0M\0README.md\0M\0scripts/mutants_gate.py\0')
        self.assertEqual({p: classify(p, s, CONFIG) for p, s in changes.items()}, {
            'tests/cli.rs': 'tests or fixtures', 'src/old.rs': 'deleted production',
            'scrl/src/new.rs': 'production', 'Cargo.lock': 'build configuration',
            'README.md': 'irrelevant', 'scripts/mutants_gate.py': 'gate'})
        self.assertEqual(classify('unknown.file', 'M', CONFIG), 'unclassified')
        self.assertEqual(classify('tests/fixture.txt', 'D', CONFIG), 'tests or fixtures')
        with self.assertRaises(GateError):
            parse_changes('R100\0only-one\0')

    def test_embedded_test_and_production_regions(self):
        old = 'fn prod() {}\n#[cfg(test)]\nmod tests {\nfn essential() {}\n}\n'
        new = 'fn prod() { 1; }\n#[cfg(test)]\nmod tests {\n}\n'
        diff = '@@ -1 +1 @@\n-fn prod() {}\n+fn prod() { 1; }\n@@ -4 +3,0 @@\n-fn essential() {}\n'
        self.assertEqual(regions(diff, old, new), (True, True))
        self.assertEqual(regions('@@ -4 +3,0 @@\n-fn essential() {}\n', old, new), (False, True))
        before = '#[cfg(test)]\nfn helper() {}\nfn public() {}\n#[cfg(test)]\nmod tests { }\n'
        after = before.replace('fn public() {}', 'fn public() { 2; }')
        self.assertEqual(regions('@@ -3 +3 @@\n-fn public() {}\n+fn public() { 2; }\n', before, after), (True, True))

    def test_diff_overlap_ignores_tool_extra_and_handles_function_span(self):
        diff = '@@ -4 +4 @@\n-old\n+new\n'
        lines = added_lines(diff)
        extra = {'name': 'unrelated', 'file': 'src/x.rs', 'span': {'start': {'line': 9}, 'end': {'line': 9}}}
        function = {'name': 'function', 'file': 'src/x.rs', 'span': {'start': {'line': 1}, 'end': {'line': 8}}}
        self.assertFalse(overlaps_changed_lines([extra], 'src/x.rs', lines))
        self.assertTrue(overlaps_changed_lines([extra, function], 'src/x.rs', lines))

    def test_partition_and_literal_regex(self):
        import re
        names = ['src/main.rs:1:2: change a+b', 'src/source.rs:3:1: replace x', 'scrl/src/session.rs:3:4: foo', 'src/align.rs:1:1: x']
        groups = {s: {n for n in names if shard_for(n) == s} for s in SHARDS}
        self.assertEqual(set.union(*groups.values()), set(names))
        self.assertEqual(sum(map(len, groups.values())), len(names))
        self.assertIsNotNone(re.fullmatch(exact_regex([names[0]]), names[0]))
        self.assertIsNone(re.fullmatch(exact_regex([names[0]]), names[0].replace('+', 'X')))

    def test_listing_rejects_invalid_or_duplicate_identities(self):
        self.assertEqual(parse_listing('[]'), [])
        self.assertEqual(parse_listing(''), [])
        for value in ('{}', '[{}]', '[{"name":"x"},{"name":"x"}]'):
            with self.subTest(value=value), self.assertRaises(GateError):
                parse_listing(value)


class ResultsTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / 'outcomes.json'

    def write(self, summary='CaughtMutant', count=1, baseline='Success'):
        kinds = {'CaughtMutant': 'caught', 'MissedMutant': 'missed', 'Timeout': 'timeout', 'Unviable': 'unviable'}
        data = {'outcomes': [{'scenario': 'Baseline', 'summary': baseline},
                {'scenario': {'Mutant': {'name': 'essential'}}, 'summary': summary}],
                'total_mutants': count, 'caught': 0, 'missed': 0, 'timeout': 0, 'unviable': 0}
        data[kinds[summary]] = 1
        self.path.write_text(json.dumps(data))

    def test_missing_bad_count_identity_and_baseline_fail(self):
        with self.assertRaises(GateError):
            read_results(self.path, ['essential'], True)
        for count, baseline in [(2, 'Success'), (1, 'Failure')]:
            self.write(count=count, baseline=baseline)
            with self.assertRaises(GateError):
                read_results(self.path, ['essential'], True)
        self.write()
        with self.assertRaises(GateError):
            read_results(self.path, ['different'], True)

    def test_equivalence_requires_exact_source_and_reason(self):
        import hashlib
        source = Path(self.directory.name) / 'source.rs'
        source.write_text('original')
        entry = {'name': 'source.rs:1:1: essential', 'file': 'source.rs', 'sha256': hashlib.sha256(source.read_bytes()).hexdigest(), 'reason': 'same observable result'}
        results = disposition({'source.rs:1:1: essential': 'missed'}, {'equivalent': [entry]}, source.parent)
        self.assertEqual(results['source.rs:1:1: essential'], 'equivalent')
        source.write_text('changed')
        with self.assertRaises(GateError):
            verdict(disposition({'source.rs:1:1: essential': 'missed'}, {'equivalent': [entry]}, source.parent))
        entry['reason'] = ''
        with self.assertRaises(GateError):
            disposition({'source.rs:1:1: essential': 'missed'}, {'equivalent': [entry]}, source.parent)

    def test_missed_and_unconfirmed_timeout_fail(self):
        for summary in ('MissedMutant', 'Timeout'):
            self.write(summary)
            with self.assertRaises(GateError):
                verdict(read_results(self.path, ['essential'], True))
        verdict({'essential': 'confirmed hang'})
        self.write()
        verdict(read_results(self.path, ['essential'], True))


# These fixtures execute the real selector against a temporary Git repository.
# The small cargo stand-in evaluates two independent arithmetic oracles; removing
# one oracle makes its mutant survive while the other remains caught.
class UnionIntegrationTests(unittest.TestCase):
    def test_deleted_test_and_covered_production_select_both(self):
        import contextlib
        import io
        import os
        import subprocess
        import sys
        from unittest.mock import patch
        import mutants_gate as gate

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ('src', 'tests', '.cargo', '.work', 'bin'):
                (root / name).mkdir()
            (root / '.gitignore').write_text('.work/\n')
            (root / 'src/main.rs').write_text('fn value() -> i32 { 1 }\n')
            (root / 'src/source.rs').write_text('fn value() -> i32 { 2 }\n')
            (root / 'tests/essential').write_text('1')
            (root / 'tests/covered').write_text('2')
            cfg = root / '.cargo/mutants-gate.toml'
            cfg.write_text('budget_seconds=30\ntimeout_seconds=1\nbuild_timeout_seconds=2\nirrelevant=["*.md"]\n[[critical]]\nfile="src/main.rs"\n')
            cargo = root / 'bin/cargo'
            cargo.write_text('#!' + sys.executable + '\n' + '''
import json, os, pathlib, re, sys
args = sys.argv[1:]
if '--list' in args and os.environ.get('FAKE_EMPTY'):
 print('[]'); sys.exit(0)
names = ['src/main.rs:1:1: replace value with 0', 'src/source.rs:1:1: replace value with 0']
if '--version' in args:
 print('cargo-mutants fixture'); sys.exit(0)
if '--in-diff' in args:
 names = names[1:]
if '--file' in args:
 files = [args[i + 1] for i, x in enumerate(args) if x == '--file']
 names = [n for n in names if any(n.startswith(f + ':') for f in files)]
if '--re' in args:
 names = [n for n in names if re.search(args[args.index('--re') + 1], n)]
if '--list' in args:
 print(json.dumps([{'name': n, 'file': n.split(':')[0], 'span': {'start': {'line': 1}, 'end': {'line': 1}}} for n in names])); sys.exit(0)
root = pathlib.Path.cwd()
results = []
for name in names:
 fixture = root / ('tests/essential' if name.startswith('src/main.rs') else 'tests/covered')
 # The mutant returns zero; each present oracle requires its literal result.
 caught = fixture.exists() and int(fixture.read_text()) != 0
 results.append((name, 'CaughtMutant' if caught else 'MissedMutant'))
out = pathlib.Path(args[args.index('--output') + 1]) / 'mutants.out'
out.mkdir(parents=True)
data = {'outcomes': [{'scenario':'Baseline','summary':'Success'}], 'total_mutants': len(names), 'caught':0, 'missed':0, 'unviable':0, 'timeout':0}
for name, result in results:
 data['outcomes'].append({'scenario': {'Mutant': {'name': name}}, 'summary':result})
 data['caught' if result == 'CaughtMutant' else 'missed'] += 1
(out / 'outcomes.json').write_text(json.dumps(data))
sys.exit(2 if data['missed'] else 0)
''')
            cargo.chmod(0o755)
            def git(*args):
                return subprocess.run(['git', *args], cwd=root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True).stdout.strip()
            git('init', '-q')
            git('config', 'user.name', 'fixture')
            git('config', 'user.email', 'fixture@example.invalid')
            git('add', 'src', 'tests', '.cargo', 'bin', '.gitignore')
            git('commit', '-qm', 'baseline')
            base = git('rev-parse', 'HEAD')
            (root / 'tests/essential').unlink()
            (root / 'src/source.rs').write_text('fn value() -> i32 { 2 + 0 }\n')
            env = {'PATH': str(root / 'bin') + os.pathsep + os.environ['PATH']}
            with patch.object(gate, 'ROOT', root), patch.object(gate, 'CONFIG', cfg), patch.object(gate, 'WORK', root / '.work/gate'), patch.dict(os.environ, env):
                source_log = io.StringIO()
                with patch.object(sys, 'argv', ['gate', base, '--shard', 'source']), contextlib.redirect_stdout(source_log):
                    gate.main()
                self.assertIn('partition cli: 1', source_log.getvalue())
                self.assertIn('partition source: 1', source_log.getvalue())
                self.assertIn('caught: src/source.rs', source_log.getvalue())
                cli_log = io.StringIO()
                with patch.object(sys, 'argv', ['gate', base, '--shard', 'cli']), contextlib.redirect_stdout(cli_log), self.assertRaisesRegex(GateError, 'missed: src/main.rs'):
                    gate.main()
                self.assertIn('tests or fixtures: tests/essential (D)', cli_log.getvalue())
                self.assertIn('missed: src/main.rs', cli_log.getvalue())
                (root / 'mystery').write_text('unknown')
                with patch.object(sys, 'argv', ['gate', base, '--shard', 'source']), contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(GateError, 'unclassified'):
                    gate.main()
                (root / 'mystery').unlink()
                with patch.dict(os.environ, {'FAKE_EMPTY': '1'}), patch.object(sys, 'argv', ['gate', base, '--shard', 'source']), contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(GateError, 'no mutants'):
                    gate.main()
                cfg.write_text(cfg.read_text().replace('budget_seconds=30', 'budget_seconds=0'))
                with patch.object(sys, 'argv', ['gate', base, '--shard', 'source']), contextlib.redirect_stdout(io.StringIO()), self.assertRaisesRegex(GateError, 'budget exhausted'):
                    gate.main()


if __name__ == '__main__':
    unittest.main()
