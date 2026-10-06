import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("marketplace_release", ROOT / "scripts/marketplace-release.py")
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class MarketplaceReleaseTest(unittest.TestCase):
    def setUp(self):
        self.run = {"head_repository": {"full_name": "muxkin/LayMesh"},
                    "path": ".github/workflows/publish-pypi.yml", "event": "workflow_dispatch",
                    "head_branch": "main", "status": "completed", "conclusion": "success",
                    "head_sha": "a" * 40}
        self.jobs = [{"name": "publish", "conclusion": "success"}]
        self.artifacts = [{"name": "reviewed-python-release"}] + [
            {"name": "editor-" + target} for target in RELEASE.TARGETS]

    def test_only_successful_same_repository_pypi_publications_are_allowed(self):
        self.assertEqual(RELEASE.validate_source(self.run, self.jobs, self.artifacts, "muxkin/LayMesh"), "a" * 40)
        for field, value in [("event", "pull_request"), ("head_branch", "feature"),
                             ("conclusion", "failure"), ("path", ".github/workflows/other.yml"),
                             ("head_repository", None),
                             ("head_repository", {"full_name": "fork/LayMesh"})]:
            run = dict(self.run, **{field: value})
            with self.assertRaises(ValueError, msg=field):
                RELEASE.validate_source(run, self.jobs, self.artifacts, "muxkin/LayMesh")
        with self.assertRaisesRegex(ValueError, "publish job"):
            RELEASE.validate_source(self.run, [{"name": "publish", "conclusion": "skipped"}],
                                    self.artifacts, "muxkin/LayMesh")

    def test_expired_or_missing_platform_artifacts_block_upload(self):
        expired = copy.deepcopy(self.artifacts)
        expired[-1]["expired"] = True
        for artifacts in [self.artifacts[:-1], expired]:
            with self.assertRaisesRegex(ValueError, "original VSIX"):
                RELEASE.validate_source(self.run, self.jobs, artifacts, "muxkin/LayMesh")

    def test_tampered_reviewed_wheels_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            wheels = [root / f"platform-{i}.whl" for i in range(5)]
            for wheel in wheels:
                wheel.write_bytes(wheel.name.encode())
            checksums = root / "SHA256SUMS"
            checksums.write_text("".join(f"{RELEASE.sha256(wheel)}  {wheel.name}\n" for wheel in wheels))
            RELEASE.verify_checksums(root, checksums)
            wheels[-1].write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                RELEASE.verify_checksums(root, checksums)

    def test_missing_preview_or_different_marketplace_packages_do_not_report_success(self):
        report = {"version": "0.3.9", "packages": [
            {"target": target, "vsix_sha256": str(i) * 64}
            for i, target in enumerate(RELEASE.TARGETS.values())]}
        versions = [{"version": report["version"], "targetPlatform": package["target"],
                     "properties": [{"key": "Microsoft.VisualStudio.Services.VsixSha256", "value": package["vsix_sha256"]}]}
                    for package in report["packages"]]
        data = {"results": [{"extensions": [{"publisher": {"publisherName": "Hyacine"},
                                             "extensionName": "laymesh-language", "versions": versions}]}]}
        self.assertEqual(RELEASE.marketplace_status(data, report), set())
        versions.pop()
        self.assertEqual(RELEASE.marketplace_status(data, report), {"win32-x64"})
        versions[0]["properties"].append({"key": "Microsoft.VisualStudio.Code.PreRelease", "value": "true"})
        with self.assertRaisesRegex(ValueError, "stable release"):
            RELEASE.marketplace_status(data, report)
        versions[0]["properties"].pop()
        versions[0]["properties"][0]["value"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "different VSIX"):
            RELEASE.marketplace_status(data, report)
