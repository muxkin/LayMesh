"""Real native animation exports and shared, consent-aware FFmpeg discovery."""
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from laymesh import (
    FFmpegDownloadRequired, LayMeshBridgeError, animate_file, animate_source,
    ensure_ffmpeg, ffmpeg_status,
)
from laymesh.bridge import _command

SOURCE = '''page = canvas(size=(100mm,65mm),stylesheet="@laymesh/animate.css",background="#ffffff")
title = page.add(text("Research results",font_size=6mm,color="#243447"),target=page.top_left,offset=(12mm,22mm))
intro = page.step(name="Title entrance",hold=1s)
intro.animate(title,class="animate__animated animate__fadeInUp")
'''


class AnimationExports(unittest.TestCase):
    def test_missing_encoder_requires_one_call_consent(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = root / "laymesh.json"
            config.write_text(json.dumps({"export": {"ffmpeg": {
                "directory": str(root / "private"), "search_system": False,
            }}}))
            self.assertFalse(ffmpeg_status(config=config)["available"])
            with patch("sys.stdin.isatty", return_value=False):
                with self.assertRaises(FFmpegDownloadRequired):
                    ensure_ffmpeg(config=config)
                with self.assertRaises(LayMeshBridgeError):
                    animate_source("page=unknown()", root / "bad.mp4", base_dir=root, config=config)
            self.assertFalse((root / "private").exists())
            config.write_text(json.dumps({"export": {"ffmpeg": {"allow_download": True}}}))
            with self.assertRaisesRegex(LayMeshBridgeError, "allow_download"):
                ffmpeg_status(config=config)

    @unittest.skipUnless(shutil.which("ffmpeg") and shutil.which("ffprobe"), "real encoder required")
    def test_screenshot_source_exports_mp4_and_gif_with_existing_encoder(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "中文 # 动画"
            root.mkdir()
            config = root / "laymesh.json"
            config.write_text(json.dumps({"export": {"ffmpeg": {
                "directory": str(root / "private"), "executable": shutil.which("ffmpeg"),
            }}}))
            file = root / "test.lay"
            file.write_text(SOURCE)
            for suffix in ("mp4", "gif"):
                with self.subTest(suffix=suffix):
                    output = root / ("结果." + suffix)
                    result = animate_file(file, output, config=config, width=160, fps=5)
                    self.assertEqual((result.width, result.height), (160, 104))
                    self.assertGreaterEqual(result.frames, 5)
                    self.assertGreaterEqual(result.duration, 1)
                    info = json.loads(subprocess.check_output([
                        "ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(output),
                    ], text=True))
                    stream = info["streams"][0]
                    self.assertEqual((stream["width"], stream["height"]), (160, 104))
                    self.assertGreater(float(info["format"]["duration"]), 0)
            inline = animate_source(SOURCE, root / "inline.mp4", base_dir=root, config=config, width=160, fps=5)
            self.assertTrue(inline.output.is_file())
            python_status = ffmpeg_status(file=file, config=config)
            cli_status = json.loads(subprocess.check_output([
                *_command(), "ffmpeg", "status", "--file", str(file), "--config", str(config), "--json",
            ], text=True))
            self.assertEqual(python_status["found"]["path"], cli_status["found"]["path"])
            self.assertTrue((root / "private/discovery.json").is_file())
            self.assertEqual(ensure_ffmpeg(file=file, config=config), Path(shutil.which("ffmpeg")).resolve())
            self.assertFalse((root / "private/install.lock").exists())
