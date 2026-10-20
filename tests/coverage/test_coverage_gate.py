"""Stdlib tests, including the actual workflow validator and real diff-cover CLI."""

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("coverage_gate", ROOT / "scripts/coverage_gate.py")
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


def workflow_validator():
    workflow = (ROOT / ".github/workflows/coverage-main.yml").read_text()
    begin = "          # BEGIN UPLOAD VALIDATOR (tests extract this exact code)\n"
    end = "          # END UPLOAD VALIDATOR"
    code = textwrap.dedent(workflow.split(begin, 1)[1].split(end, 1)[0])
    namespace = {"__name__": "coverage_validator_test"}
    exec(compile(code, str(ROOT / ".github/workflows/coverage-main.yml"), "exec"), namespace)
    return namespace["validate_upload"]


def composite_script(action, name):
    step = action.split(f"    - name: {name}\n", 1)[1].split("\n    - name:", 1)[0]
    return textwrap.dedent(step.split("      run: |\n", 1)[1])


def report_for(sources, hits=None, absolute_root=None, *, raw=False):
    output = []
    for path, identity in sources.items():
        numbers = (hits or {}).get(path, {n: 1 for n in range(1, identity["lines"] + 1)})
        source = str(absolute_root / path) if absolute_root else path
        output += ["SF:" + source]
        if raw:
            output += ["FNF:0", "FNH:0", "BRF:0", "BRH:0"]
        output += [f"DA:{n},{h}" for n, h in numbers.items()]
        output += [f"LF:{len(numbers)}", f"LH:{sum(h > 0 for h in numbers.values())}", "end_of_record"]
    return ("\n".join(output) + "\n").encode()


class RepoCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="coverage-policy-test-")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.previous = Path.cwd()
        os.chdir(self.repo)
        self.addCleanup(os.chdir, self.previous)
        self.git("init", "-q")
        self.git("config", "user.email", "coverage@example.invalid")
        self.git("config", "user.name", "Coverage fixture")
        self.write("src/lib.rs", "pub fn library() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.write("src/main.rs", "fn main() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.write("src/old.rs", "// old module\n// old second\n")
        self.base = self.commit()
        self.head = self.base
        self.artifact = self.repo / "artifact"

    def git(self, *args):
        return subprocess.run(["git", *args], capture_output=True, text=True, check=True, timeout=30).stdout.strip()

    def write(self, path, content):
        destination = self.repo / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content)

    def commit(self):
        self.git("add", "src")
        self.git("commit", "-qm", "Fixture commit")
        return self.git("rev-parse", "HEAD")

    def args(self, command, event="pull_request", **changes):
        values = {"event": event, "head": self.head,
                  "base": self.base if event == "pull_request" else self.head,
                  "repository": "example/project", "run-id": "123", "run-attempt": "2"}
        values.update(changes)
        result = [command]
        for key, value in values.items():
            result += ["--" + key, value]
        if command == "prepare":
            result += ["--report", str(self.repo / "raw.info"), "--output", str(self.artifact)]
        else:
            result += ["--artifact", str(self.artifact)]
        if command == "changed":
            executable = os.environ.get("DIFF_COVER") or shutil.which("diff-cover")
            self.assertIsNotNone(executable, "Install diff-cover==10.6.0 in a venv and set DIFF_COVER")
            result += ["--diff-cover", executable]
        return result

    def run_gate(self, args):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            code = gate.main(args)
        return code, output.getvalue()

    def prepare(self, hits=None, event="pull_request", absolute=False):
        self.sources = gate.inventory(self.head)
        self.raw = report_for(self.sources, hits, self.repo if absolute else None, raw=True)
        (self.repo / "raw.info").write_bytes(self.raw)
        result = self.run_gate(self.args("prepare", event))
        self.assertEqual(result[0], 0, result[1])
        self.canonical = (self.artifact / "lcov.info").read_bytes()
        return result

    def mutate_manifest(self, function):
        path = self.artifact / "manifest.json"
        data = json.loads(path.read_text())
        function(data)
        path.write_text(json.dumps(data))

    def replace_report(self, data):
        (self.artifact / "lcov.info").write_bytes(data)
        self.mutate_manifest(lambda m: m.update(report_sha256=hashlib.sha256(data).hexdigest()))


class PolicyTests(RepoCase):
    def test_exact_integer_threshold_and_rounding(self):
        cases = [(4, 5, 0), (799999, 1000000, 1), (8001, 10000, 0)]
        for covered, measured, want in cases:
            with self.subTest(covered=covered), contextlib.redirect_stdout(io.StringIO()) as out:
                self.assertEqual(gate.decide("Test", covered, measured), want)
                if covered == 799999:
                    self.assertIn("80.00%", out.getvalue())
                    self.assertEqual(round(100 * covered / measured), 80)

    def test_zero_is_non_numeric_na(self):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(gate.decide("Changed", 0, 0), 0)
        self.assertIn("N/A", out.getvalue())
        self.assertNotIn("100", out.getvalue())

    def test_preparation_normalizes_and_preserves_full_inventory(self):
        self.prepare(absolute=True)
        manifest = json.loads((self.artifact / "manifest.json").read_text())
        self.assertEqual(manifest["sources"], gate.inventory(self.head))
        self.assertNotIn(str(self.repo), (self.artifact / "lcov.info").read_text())
        self.assertEqual(self.run_gate(self.args("total"))[0], 0)

    def test_total_passes_exactly_eighty(self):
        self.prepare({
            "src/lib.rs": {1: 1, 2: 1, 3: 0, 4: 0},
            "src/main.rs": {1: 1, 2: 1, 3: 1, 4: 1}, "src/old.rs": {1: 1, 2: 1}})
        code, output = self.run_gate(self.args("total"))
        self.assertEqual(code, 0, output)
        self.assertIn("8/10", output)

    def test_missing_report_is_error(self):
        code, output = self.run_gate(self.args("total"))
        self.assertEqual(code, 2)
        self.assertIn("ERROR", output)

    def test_missing_base_is_error(self):
        self.prepare()
        code, output = self.run_gate(self.args("total", base="f" * 40))
        self.assertEqual(code, 2)
        self.assertIn("Git cat-file", output)

    def test_stale_manifest_fields_and_source_identity(self):
        self.prepare()
        original = (self.artifact / "manifest.json").read_bytes()
        for key, value in (("head", "f" * 40), ("base", "e" * 40), ("merge_base", "d" * 40),
                           ("run_id", "999"), ("run_attempt", "1"), ("event", "push"),
                           ("repository", "another/repo"), ("schema_version", True),
                           ("report_sha256", "0" * 64), ("sources", {})):
            with self.subTest(key=key):
                (self.artifact / "manifest.json").write_bytes(original)
                self.mutate_manifest(lambda m: m.update({key: value}))
                self.assertEqual(self.run_gate(self.args("total"))[0], 2)

    def test_new_tracked_source_is_not_exempt_from_inventory(self):
        self.write("src/pure_types.rs", "pub struct NewType;\n")
        self.head = self.commit()
        sources = gate.inventory(self.head)
        self.assertIn("src/pure_types.rs", sources)
        data = report_for({p: identity for p, identity in sources.items() if p != "src/pure_types.rs"}, raw=True)
        (self.repo / "raw.info").write_bytes(data)
        code, output = self.run_gate(self.args("prepare"))
        self.assertEqual(code, 2, output)
        self.assertIn("src/pure_types.rs", output)
        self.assertFalse(self.artifact.exists())

    def test_symlink_source_and_shallow_history_are_errors(self):
        source = self.repo / "src/old.rs"
        source.unlink()
        source.symlink_to("lib.rs")
        self.head = self.commit()
        with self.assertRaises(gate.Invalid):
            gate.inventory(self.head)
        # A shallow clone is rejected even if its head/base happen to be present.
        clone = self.repo / "shallow"
        self.git("clone", "--depth=1", self.repo.as_uri(), str(clone))
        os.chdir(clone)
        with contextlib.redirect_stderr(io.StringIO()):
            code = gate.main(self.args("total"))
        self.assertEqual(code, 2)

    def test_checkout_source_mutation_is_error(self):
        self.prepare()
        self.write("src/lib.rs", "// changed after instrumenting\n")
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)

    def test_stale_checkout_head_is_error(self):
        self.prepare()
        self.write("src/main.rs", "// new commit\n")
        self.commit()
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)

    def test_invalid_lcov_matrix(self):
        self.prepare()
        raw = (self.artifact / "lcov.info").read_bytes()
        first_record = raw.split(b"end_of_record\n", 1)[0] + b"end_of_record\n"
        cases = {
            "missing source": raw[len(first_record):],
            "unexpected source": raw.replace(b"src/old.rs", b"src/unknown.rs"),
            "absolute": raw.replace(b"SF:src/lib.rs", b"SF:/outside/src/lib.rs"),
            "traversal": raw.replace(b"SF:src/lib.rs", b"SF:src/../src/lib.rs"),
            "backslash": raw.replace(b"SF:src/lib.rs", b"SF:src\\lib.rs"),
            "duplicate source": raw + first_record,
            "duplicate line": raw.replace(b"DA:1,1\n", b"DA:1,1\nDA:1,1\n", 1),
            "duplicate summary": raw.replace(b"LF:5\n", b"LF:5\nLF:5\n", 1),
            "negative hits": raw.replace(b"DA:1,1", b"DA:1,-1", 1),
            "fraction hits": raw.replace(b"DA:1,1", b"DA:1,1.5", 1),
            "out of range": raw.replace(b"DA:1,1", b"DA:999,1", 1),
            "bad summary": raw.replace(b"LH:5", b"LH:4", 1),
            "bad LF": raw.replace(b"LF:5", b"LF:4", 1),
            "missing LF": raw.replace(b"LF:5\n", b"", 1),
            "unmeasured": raw.replace(first_record, b"SF:src/lib.rs\nLF:0\nLH:0\nend_of_record\n"),
            "no library execution": raw.replace(first_record, first_record.replace(b",1\n", b",0\n").replace(b"LH:5", b"LH:0")),
            "error text": raw + b"error: instrumentation failed\n",
            "unterminated": raw[:-len(b"end_of_record\n")],
        }
        for label, data in cases.items():
            with self.subTest(label=label):
                self.replace_report(data)
                self.assertEqual(self.run_gate(self.args("total"))[0], 2)

    def test_raw_llvm_overlap_and_instantiations_become_distinct_lines(self):
        self.prepare()
        # Authored LLVM-shaped example: two instances at line 1 plus an overlapping
        # closure at line 2. Two function groups, three detailed function records;
        # group summaries count the closure's line again, while DA lists it once.
        data = self.raw.replace(
            b"FNF:0\nFNH:0\n", b"FN:1,parent_a\nFN:1,parent_b\nFN:2,closure\n"
            b"FNDA:1,parent_a\nFNDA:1,parent_b\nFNDA:1,closure\nFNF:2\nFNH:2\n", 1)
        data = data.replace(b"LF:5\nLH:5\n", b"LF:6\nLH:6\n", 1)
        records, canonical = gate.parse_lcov(data, self.sources, raw=True)
        self.assertEqual(canonical, self.canonical)
        self.assertEqual(records["src/lib.rs"], {n: 1 for n in range(1, 6)})
        self.assertNotIn(b"FN", canonical)
        self.assertNotIn(b"BR", canonical)
        self.assertEqual(gate.parse_lcov(canonical, self.sources)[0], records)
        (self.repo / "raw.info").write_bytes(data)
        shutil.rmtree(self.artifact)
        code, output = self.run_gate(self.args("prepare"))
        self.assertEqual(code, 0, output)
        self.assertEqual((self.artifact / "lcov.info").read_bytes(), canonical)

    def test_raw_summary_pairs_are_required_and_bounded_not_detail_totals(self):
        self.prepare()
        # No extra inequality is imposed between grouped summaries and DA counts.
        data = self.raw.replace(b"LF:5\nLH:5\n", b"LF:1\nLH:0\n", 1)
        self.assertEqual(gate.parse_lcov(data, self.sources, raw=True)[1], self.canonical)
        for found, hit in ((b"LF", b"LH"), (b"FNF", b"FNH"), (b"BRF", b"BRH")):
            for missing in (found, hit):
                data = re.sub(rb"^" + missing + rb":[0-9]+\n", b"", self.raw, count=1, flags=re.M)
                with self.subTest(missing=missing), self.assertRaises(gate.Invalid):
                    gate.parse_lcov(data, self.sources, raw=True)
            data = re.sub(rb"^" + hit + rb":[0-9]+", hit + b":999", self.raw, count=1, flags=re.M)
            with self.subTest(bounds=hit), self.assertRaises(gate.Invalid):
                gate.parse_lcov(data, self.sources, raw=True)

    def test_raw_function_branch_syntax_identity_and_duplicates(self):
        self.prepare()
        data = self.raw.replace(b"FNF:0\nFNH:0\nBRF:0\nBRH:0\n",
                                b"FN:1,fixture\nFNDA:1,fixture\nFNF:1\nFNH:1\n"
                                b"BRDA:1,0,0,1\nBRF:1\nBRH:1\n", 1)
        gate.parse_lcov(data, self.sources, raw=True)
        cases = [data.replace(b"FN:1,fixture\n", b"", 1),
                 data.replace(b"FNDA:1,fixture\n", b"", 1),
                 data.replace(b"FNDA:1,fixture", b"FNDA:1,other", 1),
                 data.replace(b"FN:1,fixture", b"FN:999,fixture", 1),
                 data.replace(b"FN:1,fixture", b"FN:2,1,fixture", 1),
                 data.replace(b"FNDA:1,fixture", b"FNDA:-1,fixture", 1),
                 data.replace(b"BRDA:1,0,0,1", b"BRDA:999,0,0,1", 1),
                 data.replace(b"BRDA:1,0,0,1", b"BRDA:1,0,0,-2", 1),
                 data.replace(b"FNH:1", b"FNH:-1", 1),
                 data.replace(b"LH:5", b"LH:1.5", 1)]
        for record in (b"FN:1,fixture", b"FNDA:1,fixture", b"BRDA:1,0,0,1", b"FNF:1", b"BRH:1"):
            cases.append(data.replace(record, record + b"\n" + record, 1))
        for invalid in cases:
            with self.subTest(invalid=invalid[:80]), self.assertRaises((gate.Invalid, ValueError)):
                gate.parse_lcov(invalid, self.sources, raw=True)

    def test_canonical_consumers_reject_raw_and_all_non_line_fields(self):
        self.prepare()
        for field in (b"FN:1,fixture", b"FNDA:1,fixture", b"FNF:0", b"FNH:0",
                      b"BRDA:1,0,0,0", b"BRF:0", b"BRH:0", b"TN:"):
            data = self.canonical.replace(b"DA:1,1", field + b"\nDA:1,1", 1)
            self.replace_report(data)
            with self.subTest(field=field):
                self.assertEqual(self.run_gate(self.args("total"))[0], 2)
        self.replace_report(self.raw)
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)

    def test_artifact_extra_files_symlinks_and_duplicate_json(self):
        self.prepare()
        extra = self.artifact / "execute.py"
        extra.write_text("raise RuntimeError('must never run')")
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)
        extra.unlink()
        report = self.artifact / "lcov.info"
        report.unlink()
        report.symlink_to(self.repo / "raw.info")
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)
        report.unlink()
        report.write_bytes(self.canonical)
        manifest = self.artifact / "manifest.json"
        manifest.write_text(manifest.read_text().replace('"schema_version": 1', '"schema_version": 1, "schema_version": 1'))
        self.assertEqual(self.run_gate(self.args("total"))[0], 2)


class DiffIntegrationTests(RepoCase):
    def test_independent_total_fail_patch_pass_and_opposite(self):
        self.write("src/lib.rs", "pub fn changed() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 1, 2: 0, 3: 0, 4: 0, 5: 0},
                      "src/main.rs": {1: 1, 2: 0, 3: 0, 4: 0, 5: 0}, "src/old.rs": {1: 0, 2: 0}})
        self.assertEqual(self.run_gate(self.args("total"))[0], 1)
        self.assertEqual(self.run_gate(self.args("changed"))[0], 0)
        self.replace_report(report_for(self.sources, {"src/lib.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}}))
        self.assertEqual(self.run_gate(self.args("total"))[0], 0)
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 1, output)
        self.assertIn("0/1", output)

    def test_exact_eighty_and_below_on_real_diff(self):
        self.write("src/lib.rs", "// a\n// b\n// c\n// d\n// e\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 1, 2: 1, 3: 1, 4: 1, 5: 0}})
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("4/5", output)
        self.replace_report(report_for(self.sources, {"src/lib.rs": {1: 1, 2: 1, 3: 1, 4: 0, 5: 0}}))
        self.assertEqual(self.run_gate(self.args("changed"))[0], 1)

    def test_valid_nonmeasured_change_is_na(self):
        self.write("src/lib.rs", "pub fn library() {}\n// comment changed\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 1}})
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("N/A", output)
        self.assertNotIn("100", output)

    def test_docs_only_diff_is_na(self):
        self.write("notes.txt", "Not Rust\n")
        self.git("add", "notes.txt")
        self.git("commit", "-qm", "Documentation fixture")
        self.head = self.git("rev-parse", "HEAD")
        self.prepare()
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("N/A", output)

    def test_main_manual_na_still_validates_report(self):
        for event in ("push", "workflow_dispatch"):
            with self.subTest(event=event):
                if self.artifact.exists():
                    shutil.rmtree(self.artifact)
                self.prepare(event=event)
                code, output = self.run_gate(self.args("changed", event))
                self.assertEqual(code, 0, output)
                self.assertIn("N/A", output)
                self.assertNotIn("100", output)
                self.replace_report(b"invalid\n")
                self.assertEqual(self.run_gate(self.args("changed", event))[0], 2)

    def test_rename_destination_uses_head_coordinates(self):
        self.git("mv", "src/old.rs", "src/renamed.rs")
        self.write("src/renamed.rs", "// old module\n// old second\n// new third\n")
        self.head = self.commit()
        self.prepare({"src/renamed.rs": {1: 1, 2: 1, 3: 0}})
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 1, output)
        self.assertIn("0/1", output)

    def test_pure_rename_and_deletion_are_na(self):
        self.git("mv", "src/old.rs", "src/renamed.rs")
        self.head = self.commit()
        self.prepare()
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("N/A", output)
        shutil.rmtree(self.artifact)
        self.git("rm", "src/renamed.rs")
        self.head = self.commit()
        self.prepare()
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("N/A", output)

    def test_deleted_lines_do_not_shift_to_base_coordinates(self):
        self.write("src/lib.rs", "pub fn library() {}\n// third\n// CHANGED fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 1, 2: 1, 3: 0, 4: 1}})
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 1, output)
        self.assertIn("0/1", output)

    def test_project_config_and_working_tree_cannot_hide_lines(self):
        self.write("src/lib.rs", "pub fn changed() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}})
        self.write("pyproject.toml", '[tool.diff_cover]\nexclude = ["src/*"]\nminimum_change = 1000\n')
        self.write("other.txt", "working tree\n")
        self.assertEqual(self.run_gate(self.args("changed"))[0], 1)

    def test_tracked_binary_attribute_cannot_hide_uncovered_rust_in_real_cli(self):
        self.write(".gitattributes", "src/lib.rs -diff\n")
        self.git("add", ".gitattributes")
        self.write("src/lib.rs", "pub fn uncovered_change() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}})
        self.assertIn("Binary files", self.git("diff", self.base, self.head, "--", "src/lib.rs"))
        result = subprocess.run([sys.executable, str(ROOT / "scripts/coverage_gate.py"), *self.args("changed")],
                                capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("0/1", result.stdout)
        self.assertNotIn("N/A", result.stdout)

    def test_added_nested_source_and_diff_header_like_content(self):
        self.write("src/nested/new.rs", 'const TEXT: &str = r#"\n++ b/src/fake.rs\n"#;\n')
        self.head = self.commit()
        self.prepare({"src/nested/new.rs": {1: 1, 2: 0, 3: 1}})
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 1, output)
        self.assertIn("2/3", output)

    def test_diverged_base_uses_merge_base_not_base_tip(self):
        self.write("src/lib.rs", "pub fn changed() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.git("checkout", "--detach", self.base)
        self.write("src/main.rs", "fn base_only_change() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.base = self.commit()
        self.git("checkout", "--detach", self.head)
        self.prepare({"src/main.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}})
        manifest = json.loads((self.artifact / "manifest.json").read_text())
        self.assertNotEqual(manifest["merge_base"], self.base)
        code, output = self.run_gate(self.args("changed"))
        self.assertEqual(code, 0, output)
        self.assertIn("1/1", output)

    def test_json_omission_wrong_lines_or_integer_counts_is_error(self):
        additions = {"src/lib.rs": {1, 2}}
        records = {"src/lib.rs": {1: 1, 2: 0}}
        good = {"src_stats": {"src/lib.rs": {"covered_lines": [1], "violation_lines": [2]}},
                "total_num_lines": 2, "total_num_violations": 1, "num_changed_lines": 2}
        self.assertEqual(gate.check_diff_json(good, additions, records), (1, 2))
        mutations = [lambda r: r.update(src_stats={}), lambda r: r.update(total_num_lines=2.0),
                     lambda r: r.update(total_num_violations=0), lambda r: r.update(num_changed_lines=0),
                     lambda r: r["src_stats"]["src/lib.rs"].update(covered_lines=[1, 1]),
                     lambda r: r["src_stats"]["src/lib.rs"].update(violation_lines=[])]
        for mutate in mutations:
            report = copy.deepcopy(good)
            mutate(report)
            with self.assertRaises(gate.Invalid):
                gate.check_diff_json(report, additions, records)

    def test_nonobject_diff_json_is_actionable_error_from_main(self):
        self.write("src/lib.rs", "pub fn changed() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}})
        malformed = [None, [], "text", 1, True]
        malformed += [{"src_stats": value} for value in (None, [], "text", 1, True)]
        malformed += [{"src_stats": {"src/lib.rs": value}} for value in (None, [], "text", 1, True)]
        read_json = gate.read_json
        for report in malformed:
            def inject_diff_json(data):
                value = read_json(data)
                return value if type(value) is dict and "schema_version" in value else report
            with self.subTest(report=report), mock.patch.object(gate, "read_json", side_effect=inject_diff_json):
                code, output = self.run_gate(self.args("changed"))
            self.assertEqual(code, 2, output)
            self.assertIn("Coverage ERROR:", output)
            self.assertNotIn("N/A", output)
            self.assertNotIn("AttributeError", output)


class UploadValidatorTests(RepoCase):
    def setUp(self):
        super().setUp()
        self.validate = workflow_validator()
        self.prepare(event="push")

    def validate_current(self, **overrides):
        values = dict(directory=self.artifact, repository="example/project", run_id="123", attempt="2",
                      event="push", sha=self.head, ref="refs/heads/main")
        values.update(overrides)
        return self.validate(**values)

    def test_actual_validator_accepts_push_and_manual_data(self):
        self.validate_current()
        self.mutate_manifest(lambda m: m.update(event="workflow_dispatch"))
        self.validate_current(event="workflow_dispatch")

    def test_rejects_independently_expected_identity_or_eligibility_mismatch(self):
        for key, value in (("repository", "another/repository"), ("run_id", "124"), ("attempt", "3"),
                           ("event", "pull_request"), ("sha", "f" * 40), ("ref", "refs/heads/feature")):
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.validate_current(**{key: value})

    def test_manifest_schema_identity_inventory_and_digest(self):
        original = (self.artifact / "manifest.json").read_bytes()
        mutations = [lambda m: m.update(head="f" * 40), lambda m: m.update(base="e" * 40),
                     lambda m: m.update(merge_base="d" * 40), lambda m: m.update(run_id="999"),
                     lambda m: m.update(schema_version=True), lambda m: m.update(report_sha256="0" * 64),
                     lambda m: m.update(extra="no"), lambda m: m.pop("sources"),
                     lambda m: m["sources"].pop("src/lib.rs"),
                     lambda m: m["sources"].update({"../src/out.rs": {"lines": 1, "sha256": "a" * 64}}),
                     lambda m: m["sources"]["src/main.rs"].update(lines=True),
                     lambda m: m["sources"]["src/main.rs"].update(sha256="wrong")]
        for mutate in mutations:
            (self.artifact / "manifest.json").write_bytes(original)
            self.mutate_manifest(mutate)
            with self.assertRaises((ValueError, TypeError)):
                self.validate_current()

    def test_report_path_count_summary_and_completeness(self):
        raw = self.canonical
        cases = [b"bad", raw.replace(b"SF:src/lib.rs", b"SF:/tmp/lib.rs"),
                 raw.replace(b"SF:src/lib.rs", b"SF:src/../lib.rs"), raw + raw,
                 raw.replace(b"DA:1,1", b"DA:1,-1", 1),
                 raw.replace(b"DA:1,1\n", b"DA:1,1\nDA:1,1\n", 1),
                 raw.replace(b"LH:5", b"LH:4", 1), raw.split(b"end_of_record\n", 1)[1],
                 raw.replace(b"DA:1,1", b"DA:999,1", 1)]
        for data in cases:
            self.replace_report(data)
            with self.assertRaises(ValueError):
                self.validate_current()

    def test_actual_validator_rejects_raw_and_non_line_fields(self):
        for field in (b"FN:1,fixture", b"FNDA:1,fixture", b"FNF:0", b"FNH:0",
                      b"BRDA:1,0,0,0", b"BRF:0", b"BRH:0", b"TN:"):
            self.replace_report(self.canonical.replace(b"DA:1,1", field + b"\nDA:1,1", 1))
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate_current()
        self.replace_report(self.raw)
        with self.assertRaises(ValueError):
            self.validate_current()

    def test_unexpected_entry_symlink_missing_file_and_bounds(self):
        extra = self.artifact / "evil.py"
        extra.write_text("never execute")
        with self.assertRaises(ValueError):
            self.validate_current()
        extra.unlink()
        report = self.artifact / "lcov.info"
        report.unlink()
        report.symlink_to(self.repo / "raw.info")
        with self.assertRaises(ValueError):
            self.validate_current()
        report.unlink()
        with self.assertRaises(ValueError):
            self.validate_current()
        with report.open("wb") as stream:
            stream.truncate(50 * 1024 * 1024 + 1)
        with self.assertRaises(ValueError):
            self.validate_current()
        report.write_bytes(self.canonical)
        with (self.artifact / "manifest.json").open("wb") as stream:
            stream.truncate(2 * 1024 * 1024 + 1)
        with self.assertRaises(ValueError):
            self.validate_current()

    def test_directory_symlink_and_duplicate_json_are_rejected(self):
        link = self.repo / "artifact-link"
        link.symlink_to(self.artifact, target_is_directory=True)
        with self.assertRaises(ValueError):
            self.validate_current(directory=link)
        path = self.artifact / "manifest.json"
        path.write_text(path.read_text().replace('"schema_version": 1', '"schema_version": 1, "schema_version": 1'))
        with self.assertRaises(ValueError):
            self.validate_current()


class CompositeShellTests(RepoCase):
    def setUp(self):
        super().setUp()
        self.action = (ROOT / ".github/actions/coverage-check/action.yml").read_text()
        self.artifact = self.repo / "coverage-artifact"
        self.write("scripts/coverage_gate.py", (ROOT / "scripts/coverage_gate.py").read_text())

    def assess(self, mode, event="pull_request"):
        env = {**os.environ, "COVERAGE_CHECK": mode, "RUNNER_TEMP": str(self.repo),
               "COVERAGE_EVENT": event, "COVERAGE_HEAD": self.head,
               "COVERAGE_BASE": self.base if event == "pull_request" else self.head,
               "COVERAGE_REPOSITORY": "example/project", "COVERAGE_RUN_ID": "123",
               "COVERAGE_RUN_ATTEMPT": "2"}
        return subprocess.run(["bash", "-e", "-o", "pipefail", "-c",
                               composite_script(self.action, "Validate and enforce coverage threshold")],
                              env=env, capture_output=True, text=True, timeout=120)

    def test_actual_mode_validation_accepts_only_the_two_policies(self):
        code = composite_script(self.action, "Validate assessment mode")
        for mode in ("total", "changed", "", "prepare", "total; touch marker", "$(touch marker)", "changed\nexit 0"):
            with self.subTest(mode=mode):
                result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", code],
                                        env={**os.environ, "COVERAGE_CHECK": mode},
                                        capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0 if mode in {"total", "changed"} else 2, result)
                self.assertFalse((self.repo / "marker").exists())

    def test_actual_assessment_shell_keeps_total_and_changed_results_independent(self):
        self.write("src/lib.rs", "pub fn changed() {}\n// unchanged\n// third\n// fourth\n// fifth\n")
        self.head = self.commit()
        self.prepare({"src/lib.rs": {1: 0, 2: 1, 3: 1, 4: 1, 5: 1}})
        executable = os.environ.get("DIFF_COVER") or shutil.which("diff-cover")
        self.assertIsNotNone(executable, "Install diff-cover==10.6.0 in a venv and set DIFF_COVER")
        binary = self.repo / "coverage-venv/bin/diff-cover"
        binary.parent.mkdir(parents=True)
        binary.symlink_to(executable)
        total = self.assess("total")
        changed = self.assess("changed")
        self.assertEqual(total.returncode, 0, total.stdout + total.stderr)
        self.assertEqual(changed.returncode, 1, changed.stdout + changed.stderr)
        self.assertIn("0/1", changed.stdout)

    def test_actual_main_assessment_shell_retains_below_target_and_na_outcomes(self):
        self.prepare({"src/lib.rs": {1: 1, 2: 0, 3: 0, 4: 0, 5: 0},
                      "src/main.rs": {1: 1, 2: 0, 3: 0, 4: 0, 5: 0},
                      "src/old.rs": {1: 0, 2: 0}}, event="push")
        total = self.assess("total", "push")
        changed = self.assess("changed", "push")
        self.assertEqual(total.returncode, 1, total.stdout + total.stderr)
        self.assertEqual(changed.returncode, 0, changed.stdout + changed.stderr)
        self.assertIn("N/A", changed.stdout)
        self.assertFalse((self.repo / "coverage-venv").exists())


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.testing = (ROOT / ".github/workflows/tests.yml").read_text()
        self.coverage = (ROOT / ".github/workflows/coverage.yml").read_text()
        self.main = (ROOT / ".github/workflows/coverage-main.yml").read_text()
        self.report_action = (ROOT / ".github/actions/coverage-report/action.yml").read_text()
        self.check_action = (ROOT / ".github/actions/coverage-check/action.yml").read_text()

    @staticmethod
    def jobs(text):
        # Inspect this project's block-style jobs; actionlint validates YAML syntax.
        return dict(re.findall(r"^  ([a-z][a-z0-9_-]*):\n(.*?)(?=^  [a-z][a-z0-9_-]*:\n|\Z)",
                               text.split("\njobs:\n", 1)[1], re.M | re.S))

    def test_workflow_membership_and_action_led_names(self):
        for text, workflow, names in [
            (self.testing, "Testing", {
                "tests": "Run tests on stable Rust", "msrv": "Run tests on minimum supported Rust"}),
            (self.coverage, "Coverage", {
                "coverage": "Generate coverage report", "total": "Check total coverage",
                "changed": "Check changed-line coverage"}),
            (self.main, "Coverage on main", {
                "coverage": "Generate coverage report", "total": "Check total coverage",
                "changed": "Check changed-line coverage", "upload": "Upload main coverage to Codecov"}),
        ]:
            with self.subTest(workflow=workflow):
                self.assertTrue(text.startswith(f"name: {workflow}\n"))
                jobs = self.jobs(text)
                self.assertEqual(list(jobs), list(names))
                for job, name in names.items():
                    self.assertIn(f"    name: {name}\n", jobs[job])

    def test_events_cancellation_and_shared_safeguards(self):
        for text, events, group in [
            (self.testing, "on:\n  pull_request:\n  push:\n    branches: [main]\n  workflow_dispatch:\n",
             "testing-${{ github.event.pull_request.number || github.ref }}"),
            (self.coverage, "on:\n  pull_request:\n  workflow_dispatch:\n",
             "coverage-${{ github.event.pull_request.number || github.ref }}"),
            (self.main, "on:\n  push:\n    branches: [main]\n  workflow_dispatch:\n", "coverage-main-${{ github.ref }}"),
        ]:
            with self.subTest(group=group):
                header = text.split("\njobs:\n", 1)[0]
                trigger_block = re.search(r"^on:\n.*?(?=^\S)", header, re.M | re.S)
                self.assertIsNotNone(trigger_block)
                self.assertEqual(trigger_block[0].strip(), events.strip())
                self.assertIn(f"  group: {group}\n", header)
                self.assertIn("  cancel-in-progress: true\n", header)
                self.assertEqual(re.findall(r"^  ([a-z-]+): (read|write)$", header, re.M), [("contents", "read")])
                for forbidden in ("pull_request_target", "workflow_run", "workflow_call", "paths:",
                                  "paths-ignore:", "branches-ignore:", "secrets.", "continue-on-error:"):
                    self.assertNotIn(forbidden, text)
                for job, body in self.jobs(text).items():
                    self.assertIn("    runs-on: ubuntu-latest\n", body)
                    self.assertEqual(body.count("timeout-minutes: 30"), 1)
                    permissions = re.search(r"^    permissions:\n((?:      [a-z-]+: [a-z]+\n)+)", body, re.M)
                    self.assertIsNotNone(permissions, job)
                    expected = "      contents: read\n"
                    if job == "upload":
                        expected += "      id-token: write\n"
                    self.assertEqual(permissions[1], expected)
        local_actions = {"./.github/actions/coverage-report", "./.github/actions/coverage-check"}
        for text in (self.testing, self.coverage, self.main, self.report_action, self.check_action):
            for action in re.findall(r"uses: ([^\s]+)", text):
                if action.startswith("./"):
                    self.assertIn(action, local_actions)
                else:
                    self.assertRegex(action, r"^[A-Za-z0-9_./-]+@[0-9a-f]{40}$")

    def test_testing_is_independent_and_uses_immutable_heads(self):
        self.assertIn("  TEST_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}\n", self.testing)
        for forbidden in ("COVERAGE_", "id-token:", "llvm-cov", "uses: codecov/",
                          "actions/upload-artifact@", "actions/download-artifact@", "scripts/coverage_gate.py"):
            self.assertNotIn(forbidden, self.testing)
        for body in self.jobs(self.testing).values():
            self.assertNotIn("needs:", body)
            self.assertNotIn("    if:", body)
            self.assertIn("uses: actions/checkout@", body)
            self.assertIn("          ref: ${{ env.TEST_HEAD }}\n", body)
            self.assertIn("          persist-credentials: false\n", body)
        stable = self.jobs(self.testing)["tests"]
        self.assertIn("      RUSTUP_TOOLCHAIN: stable\n", stable)
        self.assertIn("run: rustup toolchain install stable --profile minimal\n", stable)
        self.assertIn("run: cargo test --locked\n", stable)
        self.assertIn("run: python3 -m unittest discover -s tests/coverage -v\n", stable)
        self.assertIn("DIFF_COVER: ${{ runner.temp }}/coverage-venv/bin/diff-cover\n", stable)
        self.assertEqual(self.testing.count("-m pip install 'diff-cover[toml]==10.6.0'"), 1)

    def test_pr_has_no_upload_and_entry_points_share_read_only_jobs(self):
        self.assertNotIn("id-token:", self.coverage)
        self.assertNotIn("codecov/", self.coverage)
        self.assertNotIn("Upload main coverage", self.coverage)
        for job, body in self.jobs(self.coverage).items():
            self.assertEqual(body.rstrip(), self.jobs(self.main)[job].rstrip())
        for text in (self.coverage, self.main):
            self.assertEqual(text.count("uses: ./.github/actions/coverage-report\n"), 1)
            self.assertEqual(text.count("uses: ./.github/actions/coverage-check\n"), 2)
            for job, body in self.jobs(text).items():
                if job == "upload":
                    continue
                self.assertNotIn("    if:", body)
                self.assertNotIn("run:", body)
                self.assertIn("          ref: ${{ env.COVERAGE_HEAD }}\n", body)
                self.assertIn("          fetch-depth: 0\n", body)
                self.assertIn("          persist-credentials: false\n", body)
                self.assertLess(body.index("uses: actions/checkout@"), body.index("uses: ./.github/actions/"))
                if job == "coverage":
                    self.assertNotIn("needs:", body)
                    self.assertIn("      RUSTUP_TOOLCHAIN: stable\n", body)
                else:
                    self.assertEqual(re.findall(r"^    needs: (.+)$", body, re.M), ["coverage"])
                    self.assertIn(f"          mode: {job}\n", body)
        for field in ("HEAD", "BASE"):
            self.assertIn(f"  COVERAGE_{field}: ${{{{ github.sha }}}}\n", self.main)
        self.assertIn("  COVERAGE_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}\n", self.coverage)
        self.assertIn("  COVERAGE_BASE: ${{ github.event.pull_request.base.sha || github.sha }}\n", self.coverage)

    def test_shared_actions_retain_measurement_artifacts_and_policy(self):
        self.assertEqual(self.report_action.count("cargo llvm-cov --locked --workspace --all-targets --lcov"), 1)
        artifact = "coverage-${{ github.run_id }}-${{ github.run_attempt }}-${{ env.COVERAGE_HEAD }}"
        self.assertIn(f"        name: {artifact}\n", self.report_action)
        self.assertIn(f"        name: {artifact}\n", self.check_action)
        self.assertIn("scripts/coverage_gate.py prepare", self.report_action)
        self.assertIn("--component llvm-tools-preview", self.report_action)
        self.assertIn("        tool: cargo-llvm-cov@0.9.1\n", self.report_action)
        self.assertIn("        fallback: none\n", self.report_action)
        for command in (" total ", " changed "):
            self.assertNotIn(command, self.report_action)
        self.assertLess(self.check_action.index("name: Validate assessment mode"),
                        self.check_action.index("uses: actions/download-artifact@"))
        self.assertIn("      if: inputs.mode == 'changed'\n", self.check_action)
        self.assertEqual(self.check_action.count("-m pip install 'diff-cover[toml]==10.6.0'"), 1)
        self.assertEqual(self.check_action.count("${{ inputs.mode }}"), 2)
        for action in (self.report_action, self.check_action):
            self.assertIn("  using: composite\n", action)
            self.assertEqual(action.count("      shell: bash\n"), action.count("      run:"))
            for forbidden in ("id-token:", "secrets.", "codecov/", "continue-on-error:"):
                self.assertNotIn(forbidden, action)

    def test_main_uploader_retains_producer_only_dependency_and_privilege_boundary(self):
        body = self.jobs(self.main)["upload"]
        self.assertEqual(re.findall(r"^    needs: (.+)$", body, re.M), ["coverage"])
        self.assertIn("    if: github.ref == 'refs/heads/main' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch')\n", body)
        self.assertIn("      id-token: write", body)
        for forbidden in ("uses: actions/checkout@", "uses: ./", "run: cargo", "rustup", "llvm-cov", "scripts/coverage_gate.py"):
            self.assertNotIn(forbidden, body)
        self.assertIn("          name: coverage-${{ github.run_id }}-${{ github.run_attempt }}-${{ github.sha }}\n", body)
        self.assertIn("          files: ${{ runner.temp }}/codecov-data/lcov.info\n", body)
        self.assertIn("          use_oidc: true\n", body)
        self.assertIn("override_commit: ${{ github.sha }}", body)
        self.assertIn("slug: ${{ github.repository }}", body)
        self.assertIn("plugins: noop", body)
        self.assertIn("disable_search: true", body)
        self.assertIn("disable_file_fixes: true", body)
        self.assertEqual(self.main.count("id-token: write"), 1)


if __name__ == "__main__":
    unittest.main()
