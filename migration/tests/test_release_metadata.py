"""Guard against publishing an extension and wheel with different versions."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("check_release", ROOT / "scripts/check-release.py")
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class ReleaseMetadataTest(unittest.TestCase):
    def test_current_versions_are_synchronized(self):
        self.assertEqual(RELEASE.check(), json.loads(
            (ROOT / "extensions/vscode/package.json").read_text())["version"])

    def test_extension_version_drift_blocks_release(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for name in ["Cargo.toml", "python/pyproject.toml", "release/runtime.json",
                         "extensions/vscode/package.json"]:
                destination = root / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / name, destination)
            package_path = root / "extensions/vscode/package.json"
            package = json.loads(package_path.read_text())
            package["version"] = "999.0.0"
            package_path.write_text(json.dumps(package))
            with self.assertRaisesRegex(ValueError, "VS Code extension version"):
                RELEASE.check(root)
