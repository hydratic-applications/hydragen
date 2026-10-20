"""Offline MSRV declaration, documentation, and CI handoff regressions."""

from html import escape
import importlib.util
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest import mock
from urllib.parse import urlencode


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check_rust_version.py"
SPEC = importlib.util.spec_from_file_location("check_rust_version", SCRIPT)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)

SOURCE = "https://raw.githubusercontent.com/hydratic-applications/hydragen/main/Cargo.toml"
QUERY = "$.package['rust-version']"
ALT = "Minimum supported Rust version on main"


def readme(version="1.89.0", **parameters):
    query = {
        "url": SOURCE,
        "query": QUERY,
        "label": "Minimum Rust",
        "style": "for-the-badge",
        "color": "D34516",
        "logo": "rust",
        "logoColor": "white",
    }
    query.update(parameters)
    src = escape("https://img.shields.io/badge/dynamic/toml?" + urlencode(query), quote=True)
    return (
        '# Hydragen\n\n<p align="center">\n'
        f'<a href="#build-and-run-locally"><img src="{src}" alt="{ALT}" height="22"></a>\n'
        '</p>\n\n## Build and run locally\n\n'
        f'The minimum supported Rust version (MSRV) is **{version}** for locked builds.\n'
        '\n## Development\n\nUse current stable Rust.\n'
    )


class RustVersionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="hydragen-rust-version-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.cargo = self.root / "Cargo.toml"
        self.readme = self.root / "README.md"
        self.write_version("1.89.0")
        self.readme.write_text(readme(), encoding="utf-8")

    def write_version(self, version):
        self.cargo.write_text('[package]\nname = "fixture"\nrust-version = ' + json.dumps(version) + '\n', encoding="utf-8")

    def run_cli(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *args],
            text=True, capture_output=True, timeout=10,
        )

    def assert_failed_without_output(self, result, message):
        self.assertEqual(result.returncode, 1, result)
        self.assertEqual(result.stdout, "")
        self.assertIn("Rust version check failed:", result.stderr)
        self.assertIn(message, result.stderr)

    def test_matching_checkout_and_machine_readable_output(self):
        self.assertEqual(checker.check(self.root), "1.89.0")
        result = self.run_cli("--print-version")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "1.89.0\n")
        self.assertEqual(result.stderr, "")
        result = self.run_cli()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "Rust version checks passed (MSRV 1.89.0).\n")

    def test_unsafe_or_noncanonical_versions_fail_before_any_output(self):
        values = [
            "stable", "beta", "nightly", "1.89", "1", "", "1.89.0-beta.1",
            "1.89.0+custom", "custom-toolchain", "1.89.0-x86_64-unknown-linux-gnu",
            "--help", "-1.89.0", " 1.89.0", "1.89.0 ", "1.89.0\n",
            "1.89.0\nother=nightly", "1.89.0\r\n", "1.89.0\t",
            "$(touch marker)", "1.89.0; touch marker", "`touch marker`",
            "1.89.0 && false", "1.89.0|false", "1.89.0'", '1.89.0"',
            "１.８９.０", "1.٨٩.0", "01.89.0", "1.089.0", "1.89.00",
        ]
        for value in values:
            with self.subTest(value=value):
                self.write_version(value)
                self.assert_failed_without_output(self.run_cli("--print-version"), "[package].rust-version")
        self.assertFalse((self.root / "marker").exists())

    def test_canonical_zero_components_are_valid(self):
        self.write_version("1.0.0")
        self.readme.write_text(readme("1.0.0"), encoding="utf-8")
        self.assertEqual(checker.check(self.root), "1.0.0")

    def test_missing_malformed_and_wrong_type_cargo_metadata(self):
        for content in [
            "", '[workspace]\n', '[package]\nname = "fixture"\n',
            'package = "fixture"\n', '[package]\nrust-version = 189\n',
            '[package]\nrust-version = false\n', '[package]\nrust-version = []\n',
            '[package]\nrust-version = { major = 1 }\n',
            '[package]\nrust-version = "1.89.0"\nrust-version = "1.90.0"\n',
            '[package\n',
        ]:
            with self.subTest(content=content):
                self.cargo.write_text(content, encoding="utf-8")
                self.assert_failed_without_output(self.run_cli("--print-version"), "Rust version check failed:")

    def test_missing_and_unreadable_input_paths(self):
        for path in [self.cargo, self.readme]:
            with self.subTest(path=path.name):
                saved = path.read_bytes()
                path.unlink()
                self.assert_failed_without_output(self.run_cli("--print-version"), path.name)
                path.mkdir()
                self.assert_failed_without_output(self.run_cli("--print-version"), path.name)
                path.rmdir()
                path.write_bytes(saved)

    def test_invalid_utf8_is_an_actionable_failure_not_a_traceback(self):
        for path in [self.cargo, self.readme]:
            with self.subTest(path=path.name):
                saved = path.read_bytes()
                path.write_bytes(b"\xff")
                self.assert_failed_without_output(self.run_cli("--print-version"), "decode")
                path.write_bytes(saved)

    def test_stale_absent_malformed_or_duplicated_build_prose(self):
        original = readme()
        sentence = "The minimum supported Rust version (MSRV) is **1.89.0** for locked builds."
        for text in [
            readme("1.90.0"), original.replace(sentence, "Use recent Rust."),
            original.replace("**1.89.0**", "**stable**"),
            original.replace("**1.89.0**", "1.89.0"),
            original.replace(sentence, sentence + "\n" + sentence),
            original.replace("## Build and run locally", "## Other heading"),
            original.replace("## Development", "## Build and run locally"),
            original.replace(sentence, "") + "\n" + sentence,
        ]:
            with self.subTest(text=text):
                with self.assertRaisesRegex(checker.CheckError, "matching Cargo.toml"):
                    checker.check_readme(text, "1.89.0")

    def test_wrapped_build_prose_is_accepted(self):
        checker.check_readme(readme().replace("(MSRV) is", "(MSRV)\nis"), "1.89.0")

    def test_local_branch_can_change_baseline_without_editing_dynamic_image(self):
        self.write_version("1.90.0")
        self.readme.write_text(readme("1.90.0"), encoding="utf-8")
        self.assertEqual(checker.check(self.root), "1.90.0")

    def test_each_wrong_dynamic_parameter_fails(self):
        cases = [
            {"url": SOURCE.replace("hydragen", "other")},
            {"url": SOURCE.replace("/main/", "/feature/")},
            {"url": SOURCE.replace("Cargo.toml", "other.toml")},
            {"url": SOURCE.replace("https://", "http://")},
            {"query": "$.package.edition"}, {"query": ""},
            {"label": "Rust"}, {"label": "MSRV"}, {"style": "flat-square"}, {"color": "green"},
            {"logo": ""}, {"logo": "python"}, {"logoColor": "black"},
            {"token": "not-a-real-token"}, {"message": "1.89.0"},
        ]
        for parameters in cases:
            with self.subTest(parameters=parameters):
                with self.assertRaisesRegex(checker.CheckError, "configure the MSRV shield"):
                    checker.check_readme(readme(**parameters), "1.89.0")

    def test_missing_duplicate_and_malformed_query_parameters_fail(self):
        original = readme()
        for text in [
            original.replace("&amp;color=D34516", ""),
            original.replace("&amp;color=D34516", "&amp;color=D34516&amp;color=D34516"),
            original.replace("&amp;color=D34516", "&amp;broken"),
            original.replace("&amp;logo=rust", ""),
            original.replace("&amp;logoColor=white", ""),
            original.replace("&amp;logo=rust", "&amp;logo=rust&amp;logo=rust"),
        ]:
            with self.subTest(text=text):
                with self.assertRaises(checker.CheckError):
                    checker.check_readme(text, "1.89.0")

    def test_bad_endpoints_and_static_badges_fail(self):
        for endpoint in [
            "http://img.shields.io/badge/dynamic/toml", "https://example.org/badge/dynamic/toml",
            "https://img.shields.io/badge/dynamic/json", "https://img.shields.io/badge/MSRV-1.89.0-blue",
            "https://user:password@img.shields.io/badge/dynamic/toml", "https://[",
        ]:
            with self.subTest(endpoint=endpoint):
                with self.assertRaises(checker.CheckError):
                    checker.check_readme(readme().replace("https://img.shields.io/badge/dynamic/toml", endpoint), "1.89.0")

    def test_absent_duplicate_and_commented_out_badges_fail(self):
        original = readme()
        image = original.split('<p align="center">\n')[1].split("\n</p>")[0]
        for text in [original.replace(image, ""), original.replace(image, image + image), original.replace(image, "<!--" + image + "-->")]:
            with self.subTest(text=text):
                with self.assertRaisesRegex(checker.CheckError, "exactly one"):
                    checker.check_readme(text, "1.89.0")

    def test_missing_or_misleading_alternative_text_fails(self):
        for alt in ["", "Rust", "Minimum supported Rust version: 1.89.0"]:
            with self.subTest(alt=alt):
                with self.assertRaisesRegex(checker.CheckError, "alternative text"):
                    checker.check_readme(readme().replace(ALT, alt), "1.89.0")
        with self.assertRaisesRegex(checker.CheckError, "alternative text"):
            checker.check_readme(readme().replace(f' alt="{ALT}"', ""), "1.89.0")

    def test_duplicate_image_attributes_fail(self):
        with self.assertRaisesRegex(checker.CheckError, "duplicate"):
            checker.check_readme(readme().replace('height="22"', 'height="22" height="22"'), "1.89.0")

    def test_wrong_destination_or_size_fails(self):
        for before, after in [('href="#build-and-run-locally"', 'href="Cargo.toml"'), ('height="22"', 'height="28"')]:
            with self.subTest(after=after):
                with self.assertRaisesRegex(checker.CheckError, "link the 22px MSRV shield"):
                    checker.check_readme(readme().replace(before, after), "1.89.0")

    def test_html_entities_and_self_closing_images_are_supported(self):
        text = readme().replace("&amp;", "&#38;").replace('height="22">', 'height="22" />')
        checker.check_readme(text, "1.89.0")

    def test_unrelated_images_and_links_do_not_change_badge_selection(self):
        text = '<a name="hydragen"></a><a href="LICENSE"><img src="license.svg" alt="License"></a>\n' + readme()
        checker.check_readme(text, "1.89.0")

    def test_offline_check_reads_only_cargo_and_readme(self):
        original = Path.read_text
        paths = []

        def read(path, *args, **kwargs):
            paths.append(path)
            return original(path, *args, **kwargs)

        with mock.patch.object(socket, "socket", side_effect=AssertionError("Network prohibited")), \
             mock.patch.object(socket, "getaddrinfo", side_effect=AssertionError("Network prohibited")), \
             mock.patch.object(Path, "read_text", read):
            self.assertEqual(checker.check(self.root), "1.89.0")
        self.assertEqual(paths, [self.cargo, self.readme])

    def test_readme_failure_never_emits_a_toolchain_value(self):
        self.readme.write_text(readme("1.90.0"), encoding="utf-8")
        self.assert_failed_without_output(self.run_cli("--print-version"), "matching Cargo.toml")

    def test_current_repository_matches(self):
        version = checker.check(ROOT)
        self.assertRegex(version, r"^[0-9]+\.[0-9]+\.[0-9]+$")

    def test_repository_badges_keep_balanced_groups_and_allow_wrapping(self):
        header = (ROOT / "README.md").read_text(encoding="utf-8").split('<p align="center">', 1)[1].split("</p>", 1)[0]
        rows = header.split("<br>")
        self.assertEqual(len(rows), 2)
        expected = [
            ["Status: prototype", "Language: Rust", ALT, "Dependency status", "Code coverage on main"],
            ["License: GPL-3.0-or-later", "GitHub: hydratic-applications/hydragen", "GitHub stars", "GitHub sponsors"],
        ]
        for row, labels in zip(rows, expected):
            parser = checker.LinkedImages()
            parser.feed(row)
            self.assertEqual([dict(attrs).get("alt") for attrs, _ in parser.images], labels)
            for attrs, link in parser.images:
                self.assertTrue(link)
                self.assertEqual(dict(attrs).get("height"), "22")
                self.assertNotIn("width", dict(attrs))

    def test_actual_actions_handoff_emits_only_after_successful_checks(self):
        workflow = (ROOT / ".github/workflows/tests.yml").read_text(encoding="utf-8")
        code = textwrap.dedent(workflow.split("        id: msrv\n        run: |\n", 1)[1].split("\n      - name:", 1)[0])
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / SCRIPT.name).write_bytes(SCRIPT.read_bytes())
        output = self.root / "actions-output"
        for version, prose_version, succeeds in [
            ("1.89.0", "1.89.0", True),
            ("1.89.0\nforged=nightly", "1.89.0", False),
            ("stable", "1.89.0", False),
            ("1.89.0", "1.90.0", False),
        ]:
            with self.subTest(version=version, prose_version=prose_version):
                output.unlink(missing_ok=True)
                self.write_version(version)
                self.readme.write_text(readme(prose_version), encoding="utf-8")
                result = subprocess.run(
                    ["bash", "-e", "-c", code], cwd=self.root,
                    env={**os.environ, "GITHUB_OUTPUT": str(output)},
                    capture_output=True, text=True, timeout=10,
                )
                if succeeds:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(output.read_text(), "version=1.89.0\n")
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse(output.exists())
                    self.assertIn("Rust version check failed:", result.stderr)

    def test_workflow_keeps_toolchain_selection_explicit_and_jobs_independent(self):
        workflow = (ROOT / ".github/workflows/tests.yml").read_text(encoding="utf-8")
        job = re.split(r"\n  [a-z][a-z0-9_-]*:\n", workflow.split("\n  msrv:\n", 1)[1], maxsplit=1)[0]
        self.assertIn("    name: Run tests on minimum supported Rust\n", job)
        self.assertNotIn("needs:", job)
        self.assertNotIn("secrets.", job)
        self.assertNotIn("id-token:", job)
        self.assertNotIn("1.89.0", job)
        self.assertIn('rustup toolchain install "$MSRV" --profile minimal', job)
        self.assertIn('rustc "+$MSRV" --version --verbose', job)
        self.assertIn('cargo "+$MSRV" --version', job)
        self.assertIn('cargo "+$MSRV" build --locked', job)
        self.assertIn('cargo "+$MSRV" test --locked', job)
        self.assertIn("MSRV: ${{ steps.msrv.outputs.version }}", job)
        self.assertNotIn("${{ steps.msrv.outputs.version }}", job.split("        run: |", 1)[0])
        self.assertNotIn("needs: msrv", workflow)


if __name__ == "__main__":
    unittest.main()
