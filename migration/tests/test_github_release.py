"""Release retries and checksums must never overwrite different package contents."""
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('github_release',ROOT/'scripts/github-release.py')
release=importlib.util.module_from_spec(spec);spec.loader.exec_module(release)
pypi_spec=importlib.util.spec_from_file_location('pypi_check',ROOT/'scripts/check-pypi-release.py')
pypi=importlib.util.module_from_spec(pypi_spec);pypi_spec.loader.exec_module(pypi)
class GitHubReleaseTests(unittest.TestCase):
 def fixture(self,path):
  assets={f'package-{i}.{ext}':f'{ext}-{i}'.encode() for ext in ['whl','vsix'] for i in range(5)}
  for name,data in assets.items():(path/name).write_bytes(data)
  (path/'SHA256SUMS').write_text(''.join(hashlib.sha256(data).hexdigest()+'  '+name+'\n' for name,data in sorted(assets.items())))
  return assets
 def test_tampered_missing_extra_and_traversal_assets_are_rejected(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp);assets=self.fixture(path);self.assertEqual(len(release.checksums(path)),10)
   (path/'package-0.whl').write_bytes(b'changed')
   with self.assertRaises(ValueError):release.checksums(path)
   self.fixture(path);(path/'extra').write_bytes(b'extra')
   with self.assertRaises(ValueError):release.checksums(path)
   (path/'extra').unlink();(path/'SHA256SUMS').write_text('a'*64+'  ../escape\n')
   with self.assertRaises(ValueError):release.checksums(path)
 def test_existing_conflicting_release_is_never_modified(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp);self.fixture(path)
   existing={'assets':[{'name':'package-0.whl','id':1}],'draft':False}
   with patch.object(release,'validate_tag',return_value='0.4.0'),patch.object(release.subprocess,'run',return_value=subprocess.CompletedProcess([],0,json.dumps(existing),'')),patch.object(release,'run',return_value=subprocess.CompletedProcess([],0,json.dumps({'digest':'sha256:'+'0'*64}),'')) as run:
    with self.assertRaisesRegex(ValueError,'conflicting digest'):release.publish('v0.4.0',path,'owner/repo')
    self.assertTrue(all(c.args[:3]!=('gh','release','upload') for c in run.call_args_list))
 def test_noncanonical_and_prerelease_tags_are_rejected_before_git(self):
  for tag in ['0.4.0','v0.4.0rc1','v00.4.0','v0.4.0-extra']:
   with self.assertRaises(ValueError):release.validate_tag(tag)
class PyPIRetryTests(unittest.TestCase):
 def test_partial_retry_requires_identical_existing_wheels(self):
  expected={'a.whl':'a'*64,'b.whl':'b'*64}
  partial={'urls':[{'filename':'a.whl','digests':{'sha256':'a'*64}}]}
  pypi.check_payload(expected,partial,False)
  with self.assertRaises(LookupError):pypi.check_payload(expected,partial,True)
  partial['urls'][0]['digests']['sha256']='0'*64
  with self.assertRaises(ValueError):pypi.check_payload(expected,partial,False)
 def test_complete_release_requires_all_exact_hashes(self):
  expected={'a.whl':'a'*64}
  full={'urls':[{'filename':'a.whl','digests':{'sha256':'a'*64}}]}
  pypi.check_payload(expected,full,True)
  full['urls'].append({'filename':'extra.whl','digests':{'sha256':'b'*64}})
  with self.assertRaises(ValueError):pypi.check_payload(expected,full,True)
if __name__=='__main__':unittest.main()
