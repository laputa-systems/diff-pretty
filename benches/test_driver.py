import argparse
import contextlib
import io
import sys
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
import driver


def args(**kwargs):
    return argparse.Namespace(tier=kwargs.get('tier', 'edit'), name=kwargs.get('name', []),
                              area=kwargs.get('area', []), skip=kwargs.get('skip', []))


class Selection(unittest.TestCase):
    def test_tiers_and_areas(self):
        self.assertEqual([w[0] for w in driver.select(args())], ['self-test'])
        self.assertTrue(all(w[2] == 'pager' and w[1] == 'change' for w in driver.select(args(tier='change', area=['pager']))))
        self.assertFalse(any(w[1] == 'full' for w in driver.select(args(tier='scenario'))))

    def test_errors_and_skip(self):
        for options in (args(name=['no-such-workload']), args(area=['render']), args(skip=['self-test']), args(name=['render'])):
            with self.assertRaises(ValueError):
                driver.select(options)
        self.assertNotIn('self-test', [w[0] for w in driver.select(args(tier='change', skip=['self-test']))])

    def test_exact_filter_uses_harness_module(self):
        workload = driver.select(args(tier='change', name=['render_plain_unified']))[0]
        self.assertIn('bench::render_plain_unified', driver.command(workload))

    def test_selected_preflight(self):
        with patch.object(driver.shutil, 'which', return_value=None):
            self.assertEqual(driver.preflight(driver.select(args())), [])
            self.assertTrue(driver.preflight(driver.select(args(tier='change', name=['render_plain_unified']))))

    def test_failure_and_dry_run_do_not_write_or_execute(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(driver, 'ROOT', Path(tmp)), \
                patch.object(driver.subprocess, 'run') as run, patch.object(driver.platform, 'platform', return_value='test'), contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()):
            with patch.object(sys, 'argv', ['driver', '--tier', 'change', '--name', 'render_plain_unified']), \
                    patch.object(driver.shutil, 'which', return_value=None):
                self.assertEqual(driver.main(), 1)
            with patch.object(sys, 'argv', ['driver', '--dry-run']):
                self.assertEqual(driver.main(), 0)
            self.assertFalse(list(Path(tmp).iterdir()))
            run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
