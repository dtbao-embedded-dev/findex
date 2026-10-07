#!/usr/bin/env python3
"""
Cut a release of findex.

Usage:
    python docs/scripts/tool-release.py v0.2.0 [--dry-run] [--push]
    python docs/scripts/tool-release.py --check-tag v0.2.0
    python docs/scripts/tool-release.py --selftest

What it does, in order:
    1. refuses unless the repository is in a releasable state (see check_preconditions)
    2. rewrites the version badge in README.md and every file in VERSION_FILES,
       plus the crate's own entry in a tracked Cargo.lock next to a Cargo.toml
    3. commits exactly those files plus the changelog as `chore(release): v0.2.0`
    4. tags the release commit `v0.2.0`
    5. merges the current branch into `developing`, then `developing` into `main`
    6. prints the push commands, and runs them only with --push

The release notes come from `docs/CHANGELOG/v0.2.0.md` if it exists, otherwise
from `docs/CHANGELOG/Unreleased.md`, which is renamed to `v0.2.0.md` and replaced by
a fresh skeleton. Either must hold at least one `- ` entry; see CLAUDE.md,
changelog rule.

`--check-tag` is what the release workflow runs first: it exits 1 unless every
version file and the README badge carry the version of the tag being built.

Standard library only. This file is generated; edit VERSION_FILES when a version
file moves.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

# --- edit when a version file moves -----------------------------------------
VERSION_FILES = ["package.json", "src-tauri/Cargo.toml"]
MAIN = "main"
DEV = "developing"
CHANGELOG_DIR = "docs/CHANGELOG"
UNRELEASED = "Unreleased.md"
# ----------------------------------------------------------------------------

UNRELEASED_SKELETON = """# Unreleased

> Write every change here as it lands, so the release does not have to be
> reconstructed from git history later. `docs/scripts/tool-release.py` renames this
> file to `<tag>.md` at release time and leaves a fresh copy of this skeleton behind.

### Added

### Changed

### Fixed

### Removed
"""

VERSION_RE = re.compile(r"^v\d+\.\d+\.\d+$")
BADGE_RE = re.compile(r"(img\.shields\.io/badge/version-)[^-\s)]+(-)")
REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))


class Refused(Exception):
    """A precondition failed. Nothing has been written."""


def git(*args, check=True, repo=None):
    out = subprocess.run(["git", "-C", repo or REPO] + list(args),
                         capture_output=True, text=True, encoding="utf-8")
    if check and out.returncode != 0:
        raise Refused(f"git {' '.join(args)}: {out.stderr.strip() or out.stdout.strip()}")
    return out


def tuple_of(version):
    return tuple(int(p) for p in version.lstrip("v").split("."))


def last_tag():
    out = git("describe", "--tags", "--abbrev=0", check=False)
    tag = out.stdout.strip()
    return tag if out.returncode == 0 and VERSION_RE.match(tag) else "v0.0.0"


def current_branch():
    branch = git("rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    if branch == "HEAD":
        raise Refused("detached HEAD — check out a branch before releasing")
    return branch


def branch_exists(name):
    return git("rev-parse", "--verify", f"refs/heads/{name}", check=False).returncode == 0


def changelog_path(version):
    return f"{CHANGELOG_DIR}/{version}.md"


def check_preconditions(version):
    """Every refusal happens here, before anything is written."""
    if not VERSION_RE.match(version):
        raise Refused(f"'{version}' is not semantic versioning with a v prefix (e.g. v0.2.0)")

    if git("rev-parse", "--git-dir", check=False).returncode != 0:
        raise Refused(f"{REPO} is not a git repository")

    # Untracked files are fine; modified tracked files would end up in the release
    # commit or be silently left behind.
    dirty = git("diff", "--name-only", "HEAD").stdout.strip()
    if dirty:
        raise Refused("tracked files are modified — commit or stash them first:\n  "
                      + "\n  ".join(dirty.splitlines()))

    branch = current_branch()
    if branch == MAIN:
        raise Refused(f"releases are never cut from '{MAIN}' — switch to '{DEV}' "
                      "or a work branch")
    for required in (MAIN, DEV):
        if not branch_exists(required):
            raise Refused(f"branch '{required}' does not exist locally")

    if git("rev-parse", "--verify", f"refs/tags/{version}", check=False).returncode == 0:
        raise Refused(f"tag {version} already exists")

    previous = last_tag()
    if tuple_of(version) <= tuple_of(previous):
        raise Refused(f"{version} does not increase on the last tag {previous}")

    promote = not os.path.isfile(os.path.join(REPO, changelog_path(version)))
    source = unreleased_path() if promote else changelog_path(version)
    if not os.path.isfile(os.path.join(REPO, source)):
        raise Refused(f"neither {changelog_path(version)} nor {unreleased_path()} exists — "
                      f"write the changelog from `git log {previous}..HEAD` first")
    if not re.search(r"^\s*- \S", read_text(source), re.M):
        raise Refused(f"{source} has no '- ' entry — write what changed first")

    for relpath in VERSION_FILES:
        full = os.path.join(REPO, *relpath.split("/"))
        if not os.path.isfile(full):
            raise Refused(f"version file {relpath} is missing")
        if os.path.splitext(relpath)[1] not in (".json", ".toml", ".py", ".h"):
            raise Refused(f"no rewrite rule for {relpath} — add one to set_version()")
        lock = cargo_lock_for(relpath)
        if lock and not lock_entry_re(relpath).search(read_text(lock)):
            raise Refused(f"{lock} has no [[package]] entry for the crate in {relpath}")

    return branch, previous, promote


def unreleased_path():
    return f"{CHANGELOG_DIR}/{UNRELEASED}"


def read_text(relpath):
    with open(os.path.join(REPO, *relpath.split("/")), encoding="utf-8") as fh:
        return fh.read()


def write_text(relpath, text):
    with open(os.path.join(REPO, *relpath.split("/")), "w", encoding="utf-8",
              newline="\n") as fh:
        fh.write(text)


def cargo_lock_for(relpath):
    """The tracked Cargo.lock beside a Cargo.toml version file, or None."""
    if os.path.basename(relpath) != "Cargo.toml":
        return None
    lock = "/".join(relpath.split("/")[:-1] + ["Cargo.lock"])
    tracked = git("ls-files", "--error-unmatch", "--", lock, check=False).returncode == 0
    return lock if tracked else None


def lock_entry_re(cargo_toml):
    """Matches the crate's own [[package]] block in Cargo.lock, never a dependency."""
    found = re.search(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', read_text(cargo_toml),
                      re.M | re.S)
    name = re.escape(found.group(1)) if found else "(?!)"
    return re.compile(r'(\[\[package\]\]\r?\nname = "' + name + r'"\r?\nversion = ")[^"]*')


def sync_cargo_lock(cargo_toml, version):
    lock = cargo_lock_for(cargo_toml)
    if not lock:
        return None
    text, n = lock_entry_re(cargo_toml).subn(
        lambda m: m.group(1) + version.lstrip("v"), read_text(lock), count=1)
    if not n:
        raise Refused(f"{lock} has no [[package]] entry for the crate in {cargo_toml}")
    write_text(lock, text)
    return lock


def promote_unreleased(version):
    """Unreleased.md becomes <version>.md; a fresh skeleton takes its place."""
    target = changelog_path(version)
    git("mv", "--", unreleased_path(), target)
    write_text(target, re.sub(r"\A# [^\n]*", f"# {version}", read_text(target), count=1))
    write_text(unreleased_path(), UNRELEASED_SKELETON)


def read_version(relpath):
    """The version a file currently carries, or None."""
    patterns = {
        ".json": r'"version"\s*:\s*"([^"]*)"',
        ".toml": r'^version\s*=\s*"([^"]*)"',
        ".py": r'^__version__\s*=\s*"([^"]*)"',
        ".h": r'#define\s+\w*VERSION_STRING\s+"([^"]*)"',
    }
    pattern = patterns.get(os.path.splitext(relpath)[1])
    found = pattern and re.search(pattern, read_text(relpath), re.M)
    return found.group(1) if found else None


def check_tag(version):
    """Every place the version lives must equal the tag. Returns the mismatches."""
    num = version.lstrip("v")
    problems = [f"{p} carries {read_version(p)!r}" for p in VERSION_FILES
                if read_version(p) != num]
    badge = re.search(r"img\.shields\.io/badge/version-([^-\s)]+)-", read_text("README.md"))
    if not badge or badge.group(1) != version:
        problems.append(f"README.md badge carries {badge.group(1) if badge else None!r}")
    return problems


def set_version(relpath, version):
    """Rewrite the version inside one file. Dispatch on extension, not project type."""
    num = version.lstrip("v")
    full = os.path.join(REPO, *relpath.split("/"))
    with open(full, encoding="utf-8") as fh:
        text = fh.read()
    ext = os.path.splitext(relpath)[1]

    if ext == ".json":
        text, n = re.subn(r'"version"\s*:\s*"[^"]*"', f'"version": "{num}"', text, count=1)
    elif ext == ".toml":
        text, n = re.subn(r'^version\s*=\s*"[^"]*"', f'version = "{num}"', text,
                          count=1, flags=re.M)
    elif ext == ".py":
        text, n = re.subn(r'^__version__\s*=\s*"[^"]*"', f'__version__ = "{num}"', text,
                          count=1, flags=re.M)
    elif ext == ".h":
        major, minor, patch = num.split(".")
        n = 0
        for pattern, value in (
            (r"(#define\s+\w*VERSION_MAJOR\s+)\d+", major),
            (r"(#define\s+\w*VERSION_MINOR\s+)\d+", minor),
            (r"(#define\s+\w*VERSION_PATCH\s+)\d+", patch),
            (r'(#define\s+\w*VERSION_STRING\s+")[^"]*', num),
        ):
            text, hits = re.subn(pattern, lambda m, v=value: m.group(1) + v, text, count=1)
            n += hits
        n = 1 if n == 4 else 0
    else:
        raise Refused(f"no rewrite rule for {relpath}")

    if not n:
        raise Refused(f"{relpath} has no version field this script recognises")
    with open(full, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)


def set_badge(version):
    full = os.path.join(REPO, "README.md")
    with open(full, encoding="utf-8") as fh:
        text = fh.read()
    text, n = BADGE_RE.subn(lambda m: m.group(1) + version + m.group(2), text)
    if not n:
        raise Refused("README.md has no version badge to update")
    with open(full, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(text)


def merge_chain(branch):
    """current -> developing -> main, then back to where we started."""
    steps = []
    if branch != DEV:
        steps.append((DEV, branch))
    steps.append((MAIN, DEV))
    try:
        for into, source in steps:
            git("switch", into)
            result = git("merge", "--no-ff", "-m", f"Merge {source} into {into}",
                         source, check=False)
            if result.returncode != 0:
                git("merge", "--abort", check=False)
                raise Refused(
                    f"merging {source} into {into} conflicts. The release commit and "
                    f"tag are in place; finish by hand:\n"
                    f"  git switch {into} && git merge --no-ff {source}\n"
                    + "".join(f"  git switch {i} && git merge --no-ff {s}\n"
                              for i, s in steps[steps.index((into, source)) + 1:]))
    finally:
        git("switch", branch, check=False)


def push(version, branch):
    remote = git("remote", check=False).stdout.split()
    if not remote:
        print("no git remote configured — nothing to push")
        return
    git("push", remote[0], MAIN, DEV, branch if branch != DEV else MAIN)
    git("push", remote[0], version)
    print(f"pushed {MAIN}, {DEV}, {branch} and tag {version} to {remote[0]}")


def release(version, dry_run=False, do_push=False):
    branch, previous, promote = check_preconditions(version)
    locks = [lk for lk in (cargo_lock_for(p) for p in VERSION_FILES) if lk]
    touched = VERSION_FILES + locks + ["README.md", changelog_path(version)]
    if promote:
        touched.append(unreleased_path())

    if dry_run:
        print(f"dry run: {previous} -> {version} from branch '{branch}'")
        if promote:
            print(f"  would rename {unreleased_path()} -> {changelog_path(version)}")
        for path in touched:
            print(f"  would update {path}")
        print(f"  would commit  chore(release): {version}")
        print(f"  would tag     {version}")
        if branch != DEV:
            print(f"  would merge   {branch} -> {DEV}")
        print(f"  would merge   {DEV} -> {MAIN}")
        print("  would push    " + ("yes" if do_push else "no (run with --push)"))
        return 0

    if promote:
        promote_unreleased(version)
    for relpath in VERSION_FILES:
        set_version(relpath, version)
        sync_cargo_lock(relpath, version)
    set_badge(version)

    git("add", "--", *touched)
    git("commit", "-m", f"chore(release): {version}")
    git("tag", version)
    merge_chain(branch)

    print(f"OK released {version} (was {previous})")
    print(f"   commit  chore(release): {version}")
    print(f"   merged  {branch} -> {DEV} -> {MAIN}" if branch != DEV
          else f"   merged  {DEV} -> {MAIN}")
    if do_push:
        push(version, branch)
    else:
        remote = (git("remote", check=False).stdout.split() or ["origin"])[0]
        print("   not pushed. When you are ready:")
        print(f"     git push {remote} {MAIN} {DEV} {branch}")
        print(f"     git push {remote} {version}")
    return 0


def selftest():
    """Run a full release in a throwaway repository."""
    global REPO, VERSION_FILES
    saved_repo, saved_files = REPO, VERSION_FILES
    tmp = tempfile.mkdtemp(prefix="tool_release_selftest_")
    try:
        REPO, VERSION_FILES = tmp, ["package.json"]
        ident = ["-c", "user.name=selftest", "-c", "user.email=selftest@example.com"]
        subprocess.run(["git", "-C", tmp, "init", "-b", MAIN], capture_output=True, check=True)
        os.makedirs(os.path.join(tmp, CHANGELOG_DIR))
        for path, body in (
            ("package.json", '{\n  "name": "demo",\n  "version": "0.0.0"\n}\n'),
            ("README.md", "# demo\n\n![Version](https://img.shields.io/badge/version-v0.0.0-blue)\n"),
            (f"{CHANGELOG_DIR}/v0.1.0.md", "# v0.1.0\n\n### Added\n\n- The first release.\n"),
        ):
            with open(os.path.join(tmp, path), "w", encoding="utf-8", newline="\n") as fh:
                fh.write(body)
        subprocess.run(["git", "-C", tmp, "add", "-A"], capture_output=True, check=True)
        subprocess.run(["git", "-C", tmp] + ident + ["commit", "-m", "chore: init"],
                       capture_output=True, check=True)
        subprocess.run(["git", "-C", tmp, "switch", "-c", DEV], capture_output=True, check=True)
        subprocess.run(["git", "-C", tmp, "switch", "-c", "feat/x"], capture_output=True, check=True)

        for bad, why in (("0.1.0", "missing v prefix"), ("v0.0.0", "not an increase")):
            try:
                release(bad)
                raise AssertionError(f"{bad} was accepted ({why})")
            except Refused:
                pass

        with open(os.path.join(tmp, ".git", "config"), "a", encoding="utf-8") as fh:
            fh.write("[user]\n\tname = selftest\n\temail = selftest@example.com\n")
        assert release("v0.1.0") == 0

        def show(*args):
            return subprocess.run(["git", "-C", tmp] + list(args), capture_output=True,
                                  text=True, encoding="utf-8").stdout.strip()

        assert show("rev-parse", "--abbrev-ref", "HEAD") == "feat/x", "did not return to the branch"
        assert "v0.1.0" in show("tag").split(), "tag was not created"
        subject = show("log", "-1", "--pretty=%s", "v0.1.0")
        assert subject == "chore(release): v0.1.0", f"commit subject was {subject!r}"
        assert show("branch", "--contains", "v0.1.0", "--format=%(refname:short)").split() \
            .count(MAIN) == 1, f"{MAIN} does not contain the release commit"
        with open(os.path.join(tmp, "README.md"), encoding="utf-8") as fh:
            assert "version-v0.1.0-blue" in fh.read(), "badge was not updated"
        with open(os.path.join(tmp, "package.json"), encoding="utf-8") as fh:
            assert json.load(fh)["version"] == "0.1.0", "package.json was not updated"

        try:
            release("v0.1.0")
            raise AssertionError("releasing an existing tag was accepted")
        except Refused:
            pass

        assert not check_tag("v0.1.0"), "--check-tag rejected a matching tag"
        assert check_tag("v0.2.0"), "--check-tag accepted a tag no version file carries"

        # A Rust crate: Cargo.lock carries the crate's own version too, and a build
        # after the release would rewrite it and leave the tree dirty.
        REPO = os.path.join(tmp, "crate")
        VERSION_FILES = ["package.json", "src-tauri/Cargo.toml"]
        subprocess.run(["git", "init", "-b", MAIN, REPO], capture_output=True, check=True)
        lock = ('version = 4\n\n[[package]]\nname = "demo_app"\nversion = "0.1.0"\n\n'
                '[[package]]\nname = "other"\nversion = "0.1.0"\n')
        for path, body in (
            ("package.json", '{\n  "name": "demo",\n  "version": "0.1.0"\n}\n'),
            ("README.md", "# demo\n\n![Version](https://img.shields.io/badge/version-v0.1.0-blue)\n"),
            ("src-tauri/Cargo.toml", '[package]\nname = "demo_app"\nversion = "0.1.0"\n\n'
                                     '[dependencies]\nother = { version = "0.1.0" }\n'),
            ("src-tauri/Cargo.lock", lock),
            (f"{CHANGELOG_DIR}/{UNRELEASED}", UNRELEASED_SKELETON),
        ):
            full = os.path.join(REPO, *path.split("/"))
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(body)
        with open(os.path.join(REPO, ".git", "config"), "a", encoding="utf-8") as fh:
            fh.write("[user]\n\tname = selftest\n\temail = selftest@example.com\n")
        for args in (["add", "-A"], ["commit", "-m", "chore: init"], ["tag", "v0.1.0"],
                     ["switch", "-c", DEV]):
            subprocess.run(["git", "-C", REPO] + args, capture_output=True, check=True)

        try:
            release("v0.2.0")
            raise AssertionError("a skeleton-only Unreleased.md was accepted as a changelog")
        except Refused:
            pass

        unreleased = os.path.join(REPO, CHANGELOG_DIR, UNRELEASED)
        with open(unreleased, "a", encoding="utf-8", newline="\n") as fh:
            fh.write("\n- The second release.\n")
        subprocess.run(["git", "-C", REPO, "commit", "-am", "docs: note"],
                       capture_output=True, check=True)
        assert release("v0.2.0", dry_run=True) == 0
        assert release("v0.2.0") == 0

        def read(*parts):
            with open(os.path.join(REPO, *parts), encoding="utf-8") as fh:
                return fh.read()

        assert show("-C", REPO, "status", "--porcelain") == "", "release left the tree dirty"
        lock_after = read("src-tauri", "Cargo.lock")
        assert 'name = "demo_app"\nversion = "0.2.0"' in lock_after, "Cargo.lock not synced"
        assert 'name = "other"\nversion = "0.1.0"' in lock_after, "a dependency was bumped"
        assert 'other = { version = "0.1.0" }' in read("src-tauri", "Cargo.toml"), \
            "a dependency version in Cargo.toml was rewritten"
        promoted = read(CHANGELOG_DIR, "v0.2.0.md")
        assert promoted.startswith("# v0.2.0") and "The second release." in promoted, \
            "Unreleased.md was not promoted to v0.2.0.md"
        assert read(CHANGELOG_DIR, UNRELEASED) == UNRELEASED_SKELETON, \
            "no fresh Unreleased.md skeleton was left behind"
        assert not check_tag("v0.2.0"), "--check-tag rejected the released crate"
    finally:
        REPO, VERSION_FILES = saved_repo, saved_files
        shutil.rmtree(tmp, ignore_errors=True)
    print("OK selftest")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="Cut a release.")
    parser.add_argument("version", nargs="?", help="the new version, e.g. v0.2.0")
    parser.add_argument("--dry-run", action="store_true", help="print the plan, change nothing")
    parser.add_argument("--push", action="store_true",
                        help="push branches and tag after a successful release")
    parser.add_argument("--selftest", action="store_true", help="verify this script and exit")
    parser.add_argument("--check-tag", metavar="TAG",
                        help="exit 1 unless every version file carries TAG (used by CI)")
    args = parser.parse_args(argv)

    if args.selftest:
        return selftest()
    if args.check_tag:
        problems = check_tag(args.check_tag)
        for line in problems:
            print(f"MISMATCH {args.check_tag}: {line}", file=sys.stderr)
        if not problems:
            print(f"OK every version file carries {args.check_tag}")
        return 1 if problems else 0
    if not args.version:
        parser.error("a version is required, e.g. v0.2.0")
    try:
        return release(args.version, dry_run=args.dry_run, do_push=args.push)
    except Refused as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
