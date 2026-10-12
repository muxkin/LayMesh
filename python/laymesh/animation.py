"""Animation exports and consent-aware access to the shared native FFmpeg manager."""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from .bridge import LayMeshBridgeError, _Bindings, _command, _run, _temporary_lay


class FFmpegDownloadRequired(LayMeshBridgeError):
    """An interactive confirmation or explicit one-call authorization is needed."""

    def __init__(self, status: dict):
        self.status = status
        super().__init__(
            "未找到可用 FFmpeg；请运行 laymesh ffmpeg install 确认安装，"
            "或为此次调用传入 allow_download=True。配置文件不会免除下载确认。"
        )


@dataclass(frozen=True)
class AnimationResult:
    output: Path
    frames: int
    fps: float
    duration: float
    width: int
    height: int
    page_count: int


def _manager(command: str, file: Path, format: str, config, *, authorized=False) -> dict:
    args = [*_command(), "ffmpeg", command, "--file", str(file), "--format", format, "--json"]
    if config is not None:
        args += ["--config", str(Path(config).expanduser().resolve())]
    if authorized:
        args.append("--yes")
    try:
        result = subprocess.run(
            args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
            stderr=None if command == "install" else subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace", check=False,
        )
    except OSError as error:
        raise LayMeshBridgeError(str(error)) from error
    if result.returncode:
        raise LayMeshBridgeError((result.stderr or "").strip() or "FFmpeg 管理失败；请检查配置或更新原生引擎")
    try:
        value = json.loads(result.stdout)
        if not isinstance(value.get("available"), bool):
            raise ValueError("missing status")
        return value
    except (ValueError, AttributeError) as error:
        raise LayMeshBridgeError("FFmpeg 管理器返回无效结果；请更新原生引擎") from error


def ffmpeg_status(*, file: Path | str | None = None, config=None, format="mp4") -> dict:
    """Locate existing FFmpeg without network access and record its verified path."""
    anchor = Path(file).resolve() if file else Path.cwd() / "ffmpeg.lay"
    return _manager("status", anchor, format, config)


def ensure_ffmpeg(*, file: Path | str | None = None, config=None, format="mp4", allow_download=False) -> Path:
    """Reuse existing FFmpeg; download only with interactive or one-call consent."""
    if type(allow_download) is not bool:
        raise TypeError("allow_download must be bool")
    anchor = Path(file).resolve() if file else Path.cwd() / "ffmpeg.lay"
    status = _manager("status", anchor, format, config)
    if status["available"]:
        return Path(status["found"]["path"])
    if not status["download"]["supported"]:
        raise LayMeshBridgeError("当前平台没有 FFmpeg 自动下载构建，请配置 export.ffmpeg.executable")
    authorized = allow_download
    if not authorized and sys.stdin.isatty():
        answer = input(f"未找到 FFmpeg，是否下载到 {status['directory']}？ [y/N] ")
        authorized = answer.strip().lower() in {"y", "yes", "是"}
    if not authorized:
        raise FFmpegDownloadRequired(status)
    installed = _manager("install", anchor, format, config, authorized=True)
    return Path(installed["found"]["path"])


def _animate(file: Path, output: Path | str, *, config=None, allow_download=False, **options) -> AnimationResult:
    file = file.resolve()
    output = Path(output).expanduser().resolve()
    if file.suffix not in {".lay", ".laydoc"} or not file.is_file():
        raise LayMeshBridgeError("动画输入须为已有 .lay 或 .laydoc 文件")
    if output.suffix.lower() not in {".mp4", ".gif"}:
        raise LayMeshBridgeError("动画输出须为 MP4 或 GIF")
    allowed = {"width", "fps", "reduced_motion", "crf", "preset", "background", "media", "page"}
    if set(options) - allowed:
        raise TypeError("Unknown animation options: " + ", ".join(sorted(set(options) - allowed)))
    args = ["animate", str(file), "-o", str(output)]
    for key, value in options.items():
        if value is None:
            continue
        if key == "reduced_motion":
            if type(value) is not bool:
                raise TypeError("reduced_motion must be bool")
            value = "reduce" if value else "no-preference"
        args += ["--" + key.replace("_", "-"), str(value)]
    if config is not None:
        args += ["--config", str(Path(config).expanduser().resolve())]
    # Avoid requesting an installation for syntactically invalid source.
    _run(_command(), ["validate", str(file)], file.parent)
    ensure_ffmpeg(file=file, config=config, format=output.suffix.lower()[1:], allow_download=allow_download)
    result = json.loads(_run(_command(), args, file.parent))
    return AnimationResult(
        output=Path(result["exported"]), frames=result["frames"], fps=result["fps"],
        duration=result["duration"], width=result["width"], height=result["height"], page_count=result["pageCount"],
    )


def animate_file(file: Path | str, output: Path | str, *, config=None, allow_download=False, **options) -> AnimationResult:
    """Export a .lay or .laydoc to MP4/GIF using shared FFmpeg discovery."""
    return _animate(Path(file), output, config=config, allow_download=allow_download, **options)


def animate_source(
    source: str, output: Path | str, *, namespace: Mapping[str, object] | None = None,
    base_dir: Path | str | None = None, config=None, allow_download=False, plot_dpi=192, **options,
) -> AnimationResult:
    """Export inline Lay source with the same bindings as render_source."""
    base = Path(base_dir or Path.cwd()).resolve()
    if not base.is_dir():
        raise LayMeshBridgeError("素材目录不存在：" + str(base))
    with tempfile.TemporaryDirectory(prefix="laymesh-animation-assets-") as scratch:
        binding = _Bindings(namespace or {}, _command(), base, Path(scratch), plot_dpi, None)
        file = _temporary_lay(base, binding.expand(source))
        try:
            return _animate(file, output, config=config, allow_download=allow_download, **options)
        finally:
            file.unlink(missing_ok=True)
