"""Run the phase 0 Git fixtures with Python 3.9+ and a Git executable on PATH."""

import hashlib
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import unittest


def literal_gitdir(path):
    """Escape glob metacharacters before Git serializes the config subsection."""
    escapes = {"[": "[[]", "*": "[*]", "?": "[?]", "\\": "\\\\"}
    return "".join(escapes.get(char, char) for char in path.as_posix())


class GitViability(unittest.TestCase):
    def setUp(self):
        parent = Path(tempfile.gettempdir()).resolve()
        workspace = tempfile.TemporaryDirectory(prefix="gitpilot-phase0-", dir=parent)
        self.root = Path(workspace.name).resolve()
        if self.root.parent != parent or not self.root.name.startswith("gitpilot-phase0-"):
            raise RuntimeError("Unexpected fixture directory")
        self.addCleanup(workspace.cleanup)
        self.xdg = self.root / "xdg" / "git" / "config"
        self.xdg.parent.mkdir(parents=True)
        self.global_config = self.root / ".gitconfig"
        self.manifest = self.root / "manifest.config"
        self.env = {
            key: value for key, value in os.environ.items()
            if not key.upper().startswith("GIT_") and key.upper() != "EMAIL"
        }
        self.env.update({
            "HOME": str(self.root),
            "USERPROFILE": str(self.root),
            "XDG_CONFIG_HOME": str(self.xdg.parent.parent),
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_TERMINAL_PROMPT": "0",
            "LC_ALL": "C",
        })
        self.git("config", "--file", self.global_config, "user.name", "Fixture base")
        self.git("config", "--file", self.global_config, "user.email", "base@example.test")
        self.git("config", "--file", self.global_config, "--add", "include.path", self.manifest.name)

    def git(self, *args, cwd=None, env=None, code=0):
        result = subprocess.run(
            ["git", *map(str, args)], cwd=cwd or self.root,
            env={**self.env, **(env or {})}, capture_output=True, timeout=15,
        )
        self.assertEqual(result.returncode, code, result.stderr.decode("utf-8", "replace"))
        return result.stdout

    def repo(self, relative):
        path = self.root / relative
        self.git("init", "-q", path)
        return path

    def rule(self, pattern, profile):
        config = self.root / (profile + ".config")
        self.git("config", "--file", config, "user.name", "Fixture " + profile)
        self.git("config", "--file", config, "user.email", profile + "@example.test")
        self.git("config", "--file", self.manifest, "--add", f"includeIf.{pattern}.path", config.name)

    def folder_rule(self, folder, profile):
        self.rule("gitdir:" + literal_gitdir(folder) + "/**/.git", profile)

    def email(self, repo):
        return self.git("config", "--get", "user.email", cwd=repo).strip().decode("utf-8")

    def commit(self, repo):
        self.git("commit", "--allow-empty", "-qm", "Fixture", cwd=repo)
        return self.git("log", "-1", "--format=%an <%ae> / %cn <%ce>", cwd=repo).strip()

    def test_nested_roots_override_and_future_commit(self):
        parent = self.root / "Equipo con espacios"
        self.folder_rule(parent, "general")
        self.folder_rule(parent / "Empresa", "company")
        special = self.repo("Equipo con espacios/Empresa/excepcion")
        self.rule("gitdir:" + literal_gitdir(special / ".git"), "manual")
        sibling = self.repo("Equipo con espacios/Empresa-Archivo")
        future = self.repo("Equipo con espacios/Empresa/nuevo")
        self.assertEqual(self.email(sibling), "general@example.test")
        self.assertEqual(self.email(special), "manual@example.test")
        self.assertEqual(
            self.commit(future),
            b"Fixture company <company@example.test> / Fixture company <company@example.test>",
        )

    def test_glob_metacharacters_are_literal(self):
        name = "Equipe [A] café" + (" *?\\" if os.name != "nt" else "")
        match = self.repo(name + "/proyecto")
        other = self.repo("Equipe A café/proyecto")
        self.folder_rule(self.root / name, "literal")
        self.assertEqual(self.email(match), "literal@example.test")
        self.assertEqual(self.email(other), "base@example.test")

    def test_case_sensitive_and_insensitive_patterns(self):
        repo = self.repo("Equipo/proyecto")
        pattern = literal_gitdir(self.root / "eQUIPO") + "/**/.git"
        self.rule("gitdir:" + pattern, "sensitive")
        self.assertEqual(self.email(repo), "base@example.test")
        self.rule("gitdir/i:" + pattern, "insensitive")
        self.assertEqual(self.email(repo), "insensitive@example.test")

    def test_home_xdg_and_later_relative_include(self):
        repo = self.repo("Equipo/proyecto")
        self.git("config", "--file", self.xdg, "user.email", "xdg@example.test")
        self.git("config", "--global", "user.name", "Fixture global")
        self.assertIn(b"Fixture global", self.global_config.read_bytes())
        self.folder_rule(self.root / "Equipo", "managed")
        self.assertEqual(self.email(repo), "managed@example.test")
        later = self.root / "later.config"
        self.git("config", "--file", later, "user.email", "later@example.test")
        self.git("config", "--file", self.global_config, "--add", "include.path", later.name)
        self.assertEqual(
            self.git("config", "-z", "--show-scope", "--show-origin", "--get-all", "user.email", cwd=repo)
            .split(b"\0")[2::3],
            [b"xdg@example.test", b"base@example.test", b"managed@example.test", b"later@example.test"],
        )

    def test_xdg_only_and_redirected_global(self):
        repo = self.repo("Equipo/proyecto")
        self.global_config.unlink()
        self.git("config", "--file", self.xdg, "user.email", "xdg@example.test")
        self.git("config", "--global", "user.name", "Fixture XDG")
        self.assertIn(b"Fixture XDG", self.xdg.read_bytes())
        self.assertFalse(self.global_config.exists())
        self.assertEqual(self.email(repo), "xdg@example.test")
        redirected = self.root / "redirected.config"
        self.git("config", "--file", redirected, "user.email", "custom@example.test")
        self.assertEqual(
            self.git("config", "--get", "user.email", cwd=repo, env={"GIT_CONFIG_GLOBAL": str(redirected)}).strip(),
            b"custom@example.test",
        )

    def test_local_scope_and_actual_author_environment(self):
        repo = self.repo("Equipo/proyecto")
        self.folder_rule(self.root / "Equipo", "managed")
        self.git("config", "--local", "user.email", "local@example.test", cwd=repo)
        configured = self.git("config", "-z", "--show-scope", "--show-origin", "--get", "user.email", cwd=repo)
        self.assertEqual(configured.split(b"\0")[::2], [b"local", b"local@example.test"])
        author_env = {"GIT_AUTHOR_EMAIL": "environment@example.test"}
        self.assertIn(b"<environment@example.test>", self.git("var", "GIT_AUTHOR_IDENT", cwd=repo, env=author_env))
        self.assertIn(b"<local@example.test>", self.git("var", "GIT_COMMITTER_IDENT", cwd=repo, env=author_env))

    def test_linked_worktree_move_and_gitdir_scope(self):
        repo = self.repo("Equipo/proyecto")
        base_pattern = literal_gitdir(self.root / "Equipo")
        self.rule("gitdir:" + base_pattern + "/", "broad")
        self.folder_rule(self.root / "Equipo", "ordinary")
        self.commit(repo)
        linked = self.root / "externo"
        moved = self.root / "Equipo" / "trasladado"
        self.git("worktree", "add", "-qb", "fixture", linked, cwd=repo)
        self.assertEqual(self.email(repo), "ordinary@example.test")
        self.assertEqual(self.email(linked), "broad@example.test")
        self.git("worktree", "move", linked, moved, cwd=repo)
        self.assertEqual(self.email(moved), "broad@example.test")
        self.assertIn(b"worktree ", self.git("worktree", "list", "--porcelain", "-z", cwd=repo))
        self.git("config", "extensions.worktreeConfig", "true", cwd=repo)
        self.git("config", "--worktree", "user.email", "worktree@example.test", cwd=moved)
        self.assertEqual(self.email(moved), "worktree@example.test")
        self.assertEqual(self.email(repo), "ordinary@example.test")

    def test_separate_gitdir_and_bare_reveal_native_limits(self):
        folder = self.root / "Equipo"
        folder.mkdir()
        self.folder_rule(folder, "managed")
        separate = folder / "visible"
        self.git("init", "-q", "--separate-git-dir", self.root / "metadata", separate)
        self.assertEqual(self.email(separate), "base@example.test")
        bare = folder / "bare.git"
        self.git("init", "--bare", "-q", bare)
        self.assertEqual(self.email(bare), "base@example.test")
        unusual_bare = folder / "especial" / ".git"
        self.git("init", "--bare", "-q", unusual_bare)
        self.assertEqual(self.email(unusual_bare), "managed@example.test")

    def test_submodule_uses_gitdir_below_modules(self):
        source = self.repo("source")
        self.commit(source)
        repo = self.repo("Equipo/proyecto")
        self.folder_rule(self.root / "Equipo", "managed")
        self.git("-c", "protocol.file.allow=always", "submodule", "add", "-q", source, "library", cwd=repo)
        self.assertTrue((repo / "library" / ".git").is_file())
        self.assertEqual(self.email(repo), "managed@example.test")
        self.assertEqual(self.email(repo / "library"), "base@example.test")

    def test_symlink_uses_metadata_location(self):
        repo = self.repo("Equipo/proyecto")
        self.folder_rule(self.root / "Equipo", "managed")
        alias = self.root / "alias"
        try:
            alias.symlink_to(repo, target_is_directory=True)
        except OSError as error:
            if getattr(error, "winerror", None) != 1314:
                raise
            self.skipTest(f"No directory symlink: {error.strerror}")
        self.assertEqual(self.email(alias), "managed@example.test")

    @unittest.skipUnless(os.name == "nt", "Windows junction only")
    def test_junction_uses_metadata_location(self):
        repo = self.repo("Equipo/proyecto")
        self.folder_rule(self.root / "Equipo", "managed")
        alias = self.root / "junction"
        subprocess.run(
            ["powershell.exe", "-NoProfile", "-Command",
             "New-Item -ItemType Junction -Path $env:PROBE_ALIAS -Target $env:PROBE_TARGET -ErrorAction Stop"],
            env={**self.env, "PROBE_ALIAS": str(alias), "PROBE_TARGET": str(repo)},
            capture_output=True, check=True, timeout=15,
        )
        self.assertEqual(self.email(alias), "managed@example.test")

    def test_git_lock_and_stale_preview(self):
        config = self.global_config
        before = config.read_bytes()
        fingerprint = hashlib.sha256(before).digest()
        lock = config.with_name(config.name + ".lock")
        with lock.open("xb"):
            self.git("config", "--file", config, "user.email", "race@example.test", code=255)
            self.assertEqual(config.read_bytes(), before)
        lock.unlink()
        self.git("config", "--file", config, "user.email", "external@example.test")
        self.assertNotEqual(hashlib.sha256(config.read_bytes()).digest(), fingerprint)

    def test_replace_restore_and_later_external_edit(self):
        config = self.global_config
        before = config.read_bytes()
        repo = self.repo("proyecto")
        candidate = config.with_name("candidate.config")
        candidate.write_bytes(before)
        self.git("config", "--file", candidate, "user.email", "changed@example.test")
        applied = candidate.read_bytes()
        os.replace(candidate, config)
        self.assertEqual(self.email(repo), "changed@example.test")
        self.assertEqual(config.read_bytes(), applied)
        candidate.write_bytes(before)
        os.replace(candidate, config)
        self.assertEqual(self.email(repo), "base@example.test")
        self.git("config", "--file", config, "user.email", "later@example.test")
        self.assertNotIn(config.read_bytes(), (before, applied))


if __name__ == "__main__":
    git_version = subprocess.check_output(["git", "--version"], timeout=15, text=True).strip()
    print(f"{platform.system()} {platform.release()} ({platform.machine()}) | {git_version}", flush=True)
    unittest.main(verbosity=2)
