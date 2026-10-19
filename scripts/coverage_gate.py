#!/usr/bin/env python3
"""Run-bound LCOV data validation and exact, independent coverage policy gates."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile

MAX_REPORT = 50 * 1024 * 1024
MAX_MANIFEST = 2 * 1024 * 1024
SHA = re.compile(r"[0-9a-f]{40}")
SOURCE = re.compile(r"src/(?:[A-Za-z0-9_-]+/)*[A-Za-z0-9_-]+\.rs")


class Invalid(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Invalid(message)


def natural(value):
    require(re.fullmatch(r"0|[1-9][0-9]*", value) is not None,
            f"Invalid non-negative integer: {value!r}")
    return int(value)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(data):
    return json.loads(data, object_pairs_hook=unique_object)


def git(*args):
    result = subprocess.run(["git", *args], capture_output=True, check=False, timeout=60)
    require(result.returncode == 0,
            f"Git {' '.join(args)} failed: {result.stderr.decode(errors='replace').strip()}")
    return result.stdout


def context(args):
    require(SHA.fullmatch(args.head) is not None, "--head must be an immutable full commit SHA")
    require(SHA.fullmatch(args.base) is not None, "--base must be an immutable full commit SHA")
    require(args.event in {"pull_request", "push", "workflow_dispatch"}, "Unsupported event")
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository), "Invalid repository")
    require(re.fullmatch(r"[1-9][0-9]*", args.run_id), "Invalid run ID")
    require(re.fullmatch(r"[1-9][0-9]*", args.run_attempt), "Invalid run attempt")
    require(git("rev-parse", "HEAD").decode().strip() == args.head, "Checkout HEAD differs from expected head")
    require(git("rev-parse", "--is-shallow-repository").strip() == b"false",
            "Complete history is required; fetch immutable head and base with fetch-depth: 0")
    for commit in (args.head, args.base):
        require(git("cat-file", "-t", commit).strip() == b"commit", f"Missing commit: {commit}")
    if args.event != "pull_request":
        require(args.base == args.head, "Non-PR policy requires base=head")
    bases = git("merge-base", "--all", args.base, args.head).decode().splitlines()
    require(len(bases) == 1, "Expected exactly one merge base; fetch full history or resolve ambiguous ancestry")
    return {"schema_version": 1, "repository": args.repository, "run_id": args.run_id,
            "run_attempt": args.run_attempt, "event": args.event, "head": args.head,
            "base": args.base, "merge_base": bases[0]}


def inventory(head):
    entries = git("ls-tree", "-rz", head, "--", "src/").split(b"\0")
    result = {}
    for entry in filter(None, entries):
        metadata, raw_path = entry.split(b"\t", 1)
        path = raw_path.decode("utf-8")
        if not path.endswith(".rs"):
            continue
        require(SOURCE.fullmatch(path), f"Unsafe tracked Rust source path: {path!r}")
        mode, kind, oid = metadata.decode().split()
        require(mode in {"100644", "100755"} and kind == "blob", f"Source is not a regular file: {path}")
        data = git("cat-file", "blob", oid)
        local = Path(path)
        require(not any(p.is_symlink() for p in [local, *local.parents]), f"Symlink in source path: {path}")
        require(local.is_file() and local.read_bytes() == data, f"Source differs from immutable head: {path}")
        result[path] = {"sha256": digest(data), "lines": len(data.splitlines())}
    require(result, "No tracked src/**/*.rs files found")
    require({"src/lib.rs", "src/main.rs"} <= result.keys(), "Expected library and CLI source files")
    return dict(sorted(result.items()))


def source_path(value, root=None):
    if root is not None and value.startswith(str(root) + "/"):
        value = value[len(str(root)) + 1:]
    require(SOURCE.fullmatch(value), f"Unsafe or unmappable LCOV source path: {value!r}")
    return value


def parse_lcov(data, sources, root=None, *, raw=False):
    """Validate canonical line-only data, or raw LLVM data during preparation only.

    LLVM's raw summaries count function groups, whose lines can overlap; they do
    not necessarily equal file-level DA records or function instantiations.
    Raw mode checks summary pairs, not unproven relationships to detailed records.
    Both modes return every distinct DA entry and canonical line-only LCOV.
    No source exemptions: even an unmeasured tracked module needs a record with
    measured lines. An omitted module is an actionable error, not a smaller scope.
    """
    require(0 < len(data) <= MAX_REPORT, "Missing, empty or oversized LCOV report")
    records, output, current = {}, [], None
    for line in data.decode("utf-8").splitlines():
        if raw and line.startswith("TN:") and current is None:
            require(line == "TN:", "Unexpected named LCOV test record")
            continue
        if line.startswith("SF:"):
            require(current is None, "Unterminated LCOV record")
            path = source_path(line[3:], root if raw else None)
            require(path in sources, f"Unexpected/stale LCOV source: {path}")
            require(path not in records, f"Duplicate LCOV source record: {path}")
            current = {"path": path, "lines": {}, "summaries": {}, "fn": {}, "hits": {}, "branches": {}}
            continue
        require(current is not None, f"LCOV data outside source record: {line!r}")
        if line == "end_of_record":
            s, lines = current["summaries"], current["lines"]
            require(lines, f"No measured lines for {current['path']}; check instrumentation, do not exempt source")
            if raw:
                for found, hit in (("LF", "LH"), ("FNF", "FNH"), ("BRF", "BRH")):
                    require(found in s and hit in s, f"Missing raw LLVM {found}/{hit} summary pair")
                    require(s[hit] <= s[found], f"Invalid raw LLVM {found}/{hit} summary bounds")
                require(current["fn"].keys() == current["hits"].keys(), "Function definitions/hits disagree")
            else:
                require(s.get("LF") == len(lines) and s.get("LH") == sum(h > 0 for h in lines.values()),
                        f"Canonical LCOV line summaries disagree for {current['path']}")
            records[current["path"]] = lines
            output.append("SF:" + current["path"])
            output.extend(f"DA:{number},{hits}" for number, hits in lines.items())
            output.extend((f"LF:{len(lines)}", f"LH:{sum(h > 0 for h in lines.values())}", "end_of_record"))
            current = None
        else:
            tag, sep, value = line.partition(":")
            require(sep, f"Malformed LCOV line: {line!r}")
            if tag == "DA":
                parts = value.split(",")
                require(len(parts) == 2, "DA requires line and hit count (no unchecked checksums)")
                number, hits = map(natural, parts)
                require(0 < number <= sources[current["path"]]["lines"], f"LCOV line outside source: {line}")
                require(number not in current["lines"], f"Duplicate DA line: {line}")
                current["lines"][number] = hits
            elif tag in {"LF", "LH"} or (raw and tag in {"FNF", "FNH", "BRF", "BRH"}):
                require(tag not in current["summaries"], f"Duplicate summary: {tag}")
                current["summaries"][tag] = natural(value)
            elif raw and tag == "FN":
                start, name = value.split(",", 1)
                start = natural(start)
                end = start
                # LLVM may include an end-line before the function name.
                if re.match(r"[0-9]+,", name):
                    end, name = name.split(",", 1)
                    end = natural(end)
                require(0 < start <= end <= sources[current["path"]]["lines"]
                        and re.fullmatch(r"[^\x00-\x1f\x7f]+", name) and name not in current["fn"],
                        "Invalid/duplicate function definition or range")
                current["fn"][name] = start
            elif raw and tag == "FNDA":
                hits, name = value.split(",", 1)
                require(re.fullmatch(r"[^\x00-\x1f\x7f]+", name) and name not in current["hits"],
                        "Invalid/duplicate function hits")
                current["hits"][name] = natural(hits)
            elif raw and tag == "BRDA":
                number, block, branch, hits = value.split(",")
                key = (natural(number), natural(block), natural(branch))
                require(0 < key[0] <= sources[current["path"]]["lines"] and key not in current["branches"],
                        "Invalid/duplicate branch")
                current["branches"][key] = 0 if hits == "-" else natural(hits)
            else:
                raise Invalid(f"Unsupported LCOV field: {tag}")
    require(current is None, "Unterminated LCOV report")
    missing = sources.keys() - records.keys()
    require(not missing, "Missing LCOV source records: " + ", ".join(sorted(missing)) + "; check instrumentation")
    for path in ("src/lib.rs", "src/main.rs"):
        require(any(records[path].values()), f"No executed lines for {path}")
    return records, ("\n".join(output) + "\n").encode()


def artifact_file(directory, name, limit):
    require(directory.is_dir() and not directory.is_symlink(), "Artifact directory must be a real directory")
    require({p.name for p in directory.iterdir()} == {"lcov.info", "manifest.json"},
            "Artifact must contain exactly lcov.info and manifest.json")
    path = directory / name
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= limit, f"Invalid artifact file: {name}")
    return path.read_bytes()


def load_artifact(args, expected, sources):
    directory = Path(args.artifact)
    data = artifact_file(directory, "lcov.info", MAX_REPORT)
    manifest = read_json(artifact_file(directory, "manifest.json", MAX_MANIFEST))
    require(type(manifest) is dict and manifest.keys() == (expected.keys() | {"report_sha256", "sources"}),
            "Invalid manifest schema/fields")
    for key, value in expected.items():
        require(type(manifest[key]) is type(value) and manifest[key] == value, f"Stale/mismatched manifest {key}")
    require(manifest["report_sha256"] == digest(data), "LCOV digest mismatch")
    require(type(manifest["sources"]) is dict, "Invalid source inventory")
    for path, identity in manifest["sources"].items():
        require(type(identity) is dict and identity.keys() == {"sha256", "lines"}
                and type(identity["lines"]) is int and type(identity["sha256"]) is str,
                f"Invalid source identity: {path}")
    require(manifest["sources"] == sources, "Source inventory/identity mismatch")
    records, normalized = parse_lcov(data, sources)
    require(data == normalized, "Artifact LCOV is not canonical repository-relative data")
    return records


def decide(label, covered, measured):
    require(type(covered) is int and type(measured) is int and 0 <= covered <= measured,
            "Invalid policy counts")
    if measured == 0:
        print(f"{label}: N/A — no applicable measured head lines (valid report and comparison).")
        return 0
    passed = 5 * covered >= 4 * measured
    print(f"{label}: {'PASS' if passed else 'FAIL'} — {covered}/{measured} lines ({100 * covered / measured:.2f}%); at least 80% required.")
    return 0 if passed else 1


def changed_lines(diff):
    """Read head coordinates, not removed/base coordinates, from our explicit Git diff."""
    result, path, number, deleted = {}, None, None, False
    for line in diff.splitlines():
        if line.startswith("diff --git "):
            path, number, deleted = None, None, False
        elif line.startswith("+++ ") and number is None:
            target = line[4:]
            deleted = target == "/dev/null"
            if not deleted:
                require(target.startswith("b/"), "Unexpected Git diff head path")
                path = source_path(target[2:])
                result.setdefault(path, set())
        elif line.startswith("@@ "):
            if deleted:
                continue
            match = re.match(r"@@ -[0-9]+(?:,[0-9]+)? \+([0-9]+)(?:,[0-9]+)? @@", line)
            require(match is not None and path is not None, "Malformed head diff hunk")
            number = int(match[1])
        elif number is not None:
            if line.startswith("+"):
                result[path].add(number)
                number += 1
            elif line.startswith(" "):
                number += 1
    return result


def check_diff_json(report, additions, records):
    require(type(report) is dict, "diff-cover JSON root must be an object")
    expected = {}
    for path, lines in additions.items():
        require(path in records, f"Changed source missing from LCOV: {path}")
        measured = lines & records[path].keys()
        if measured:
            expected[path] = (sorted(n for n in measured if records[path][n] > 0),
                              sorted(n for n in measured if records[path][n] == 0))
    stats = report.get("src_stats")
    require(type(stats) is dict and stats.keys() == expected.keys(), "diff-cover omitted or added measured source paths")
    covered = measured = 0
    for path, (hits, misses) in expected.items():
        require(type(stats[path]) is dict, f"diff-cover source statistics must be an object: {path}")
        for key, want in (("covered_lines", hits), ("violation_lines", misses)):
            got = stats[path].get(key)
            require(type(got) is list and all(type(n) is int for n in got) and got == want,
                    f"diff-cover {key} disagrees with LCOV/head diff: {path}")
        covered += len(hits)
        measured += len(hits) + len(misses)
    for key, want in (("total_num_lines", measured), ("total_num_violations", measured - covered),
                      ("num_changed_lines", sum(map(len, additions.values())))):
        require(type(report.get(key)) is int and report[key] == want, f"diff-cover invalid {key}")
    return covered, measured


def assess_changed(args, expected, records):
    if args.event != "pull_request":
        print("Changed-line coverage: N/A — policy applies to pull requests only; valid report verified.")
        return 0
    version = subprocess.run([args.diff_cover, "--version"], capture_output=True, text=True, check=True, timeout=30)
    require(version.stdout.strip() == "diff-cover 10.6.0", "Install exactly diff-cover==10.6.0 in an isolated venv")
    diff = git("-c", "core.quotePath=false", "diff", "--text", "--no-ext-diff", "--no-textconv", "--no-color",
               "--find-renames", "--unified=0", expected["merge_base"], args.head, "--", ":(glob)src/**/*.rs").decode()
    additions = changed_lines(diff)
    with tempfile.TemporaryDirectory(prefix="coverage-diff-") as directory:
        temp = Path(directory)
        (temp / "diff.patch").write_text(diff)
        (temp / "empty.toml").write_text("[tool.diff_cover]\nfail_under = 0\n")
        # Explicit config prevents project/working-tree config, exclusions and minimum-change overrides.
        result = subprocess.run([args.diff_cover, str(Path(args.artifact).resolve() / "lcov.info"),
                        "--diff-file", str(temp / "diff.patch"), "--config-file", str(temp / "empty.toml"),
                        "--format", f"json:{temp / 'diff.json'}", "--fail-under", "0", "--minimum-change", "0",
                        "--ignore-staged", "--ignore-unstaged", "--quiet"], capture_output=True, text=True, timeout=120)
        require(result.returncode == 0, f"diff-cover failed: {result.stderr.strip()}")
        covered, measured = check_diff_json(read_json((temp / "diff.json").read_bytes()), additions, records)
    return decide("Changed-line coverage", covered, measured)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("prepare", "total", "changed"):
        sub = subparsers.add_parser(command)
        for name in ("event", "head", "base", "repository", "run-id", "run-attempt"):
            sub.add_argument("--" + name, required=True)
        if command == "prepare":
            sub.add_argument("--report", required=True)
            sub.add_argument("--output", required=True)
        else:
            sub.add_argument("--artifact", required=True)
        if command == "changed":
            sub.add_argument("--diff-cover", default="diff-cover", help="Isolated diff-cover 10.6.0 executable")
    args = parser.parse_args(argv)
    try:
        expected = context(args)
        sources = inventory(args.head)
        if args.command == "prepare":
            root = Path(git("rev-parse", "--show-toplevel").decode().strip())
            _, normalized = parse_lcov(Path(args.report).read_bytes(), sources, root, raw=True)
            parse_lcov(normalized, sources)  # Consumers receive only the strict canonical format.
            manifest = {**expected, "report_sha256": digest(normalized), "sources": sources}
            output = Path(args.output)
            output.mkdir(parents=True, exist_ok=False)
            (output / "lcov.info").write_bytes(normalized)
            (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
            print(f"Prepared valid report for {len(sources)} tracked Rust sources; no percentage gate applied.")
            return 0
        records = load_artifact(args, expected, sources)
        if args.command == "changed":
            return assess_changed(args, expected, records)
        counts = [hits for lines in records.values() for hits in lines.values()]
        require(counts, "Total coverage requires measured lines")
        return decide("Total coverage", sum(n > 0 for n in counts), len(counts))
    except (Invalid, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print(f"Coverage ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
