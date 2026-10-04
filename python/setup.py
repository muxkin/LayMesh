"""Platform-wheel policy. The release builder stages the audited private runtime."""
import json
from pathlib import Path

from setuptools import setup
from setuptools.command.bdist_wheel import bdist_wheel

ROOT = Path(__file__).parent
VENDOR = ROOT / "laymesh/_vendor"


class RuntimeWheel(bdist_wheel):
    def finalize_options(self):
        super().finalize_options()
        if VENDOR.is_dir():
            self.root_is_pure = False

    def run(self):
        if not (VENDOR / "manifest.json").is_file():
            raise RuntimeError("Release wheels require the private runtime. Use scripts/build-python-wheel.py; editable source installs remain supported.")
        super().run()

    def get_tag(self):
        if VENDOR.is_dir():
            manifest = json.loads((VENDOR / "manifest.json").read_text(encoding="utf-8"))
            return "py3", "none", manifest["wheel_tag"]
        return super().get_tag()


setup(
    cmdclass={"bdist_wheel": RuntimeWheel},
    package_data={"laymesh": [p.relative_to(ROOT / "laymesh").as_posix() for p in VENDOR.rglob("*") if p.is_file()]},
)
