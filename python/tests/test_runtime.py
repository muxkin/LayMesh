import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from laymesh import bridge
from laymesh import _runtime


class RuntimeTest(unittest.TestCase):
    def test_private_runtime_precedes_unrelated_path_command(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(bridge, "bundled_command", return_value=["private-cli"]), patch.object(bridge.shutil, "which", side_effect=AssertionError("PATH must not be used")):
            self.assertEqual(bridge._command(), ["private-cli"])

    def test_explicit_js_override_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            cli = Path(temp) / "custom.js"
            cli.write_text("", encoding="utf-8")
            with patch.dict(os.environ, {"LAYMESH_CLI": str(cli)}), patch.object(bridge, "bundled_command", return_value=["private-cli"]):
                with self.assertRaisesRegex(bridge.LayMeshBridgeError, "native Rust"):
                    bridge._command()

    def test_corrupt_runtime_has_reinstallation_guidance(self):
        with tempfile.TemporaryDirectory() as temp:
            package = Path(temp) / "laymesh"
            (package / "_vendor").mkdir(parents=True)
            with patch.object(_runtime, "__file__", str(package / "_runtime.py")):
                with self.assertRaisesRegex(RuntimeError, "Reinstall"):
                    _runtime.bundled_command()

    def test_cli_propagates_failure_without_recursing(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(_runtime, "bundled_command", return_value=["private-cli"]), patch.object(_runtime.subprocess, "call", return_value=2) as call, patch.object(_runtime.sys, "argv", ["laymesh", "validate", "bad.lay"]):
            self.assertEqual(_runtime.main(), 2)
            call.assert_called_once_with(["private-cli", "validate", "bad.lay"])
