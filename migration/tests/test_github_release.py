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
    self.assertTrue(all('--method' not in c.args for c in run.call_args_list))
 def test_noncanonical_and_prerelease_tags_are_rejected_before_git(self):
  for tag in ['0.4.0','v0.4.0rc1','v00.4.0','v0.4.0-extra']:
   with self.assertRaises(ValueError):release.validate_tag(tag)
 def test_new_and_existing_drafts_use_numeric_endpoint_before_publication(self):
  for resume in [False,True]:
   with self.subTest(resume=resume),tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp);assets=self.fixture(path)
    wanted={**{name:hashlib.sha256(data).hexdigest() for name,data in assets.items()},'SHA256SUMS':hashlib.sha256((path/'SHA256SUMS').read_bytes()).hexdigest()}
    uploaded=[{'id':i,'name':name,'digest':'sha256:'+digest} for i,(name,digest) in enumerate(wanted.items(),1)]
    draft={'id':77,'tag_name':'v0.5.0','draft':True,'assets':uploaded if resume else []}
    final={**draft,'assets':uploaded}
    calls=[];listings=iter([[draft]] if resume else [[]])
    def fake_run(*args,**kwargs):
     calls.append(args)
     if args[:2]==('gh','api'):
      endpoint=args[2]
      if '--method' in args:
       method=args[args.index('--method')+1]
       body=Path(args[args.index('--input')+1])
       if endpoint.startswith('https://uploads.github.com/'):
        self.assertEqual(method,'POST');self.assertIn('Content-Type: application/octet-stream',args)
        self.assertTrue(body.is_file());payload={}
       elif endpoint.endswith('/releases'):
        self.assertEqual(method,'POST');self.assertEqual(json.loads(body.read_text())['tag_name'],'v0.5.0');payload=draft
       else:
        self.assertEqual(method,'PATCH');self.assertEqual(json.loads(body.read_text()),{'draft':False});payload=final
      elif endpoint.endswith('releases?per_page=100'):payload=next(listings)
      elif endpoint.endswith('releases/77'):payload=final
      elif '/releases/assets/' in endpoint:payload=next(a for a in uploaded if str(a['id'])==endpoint.rsplit('/',1)[1])
      else:self.fail('Unexpected API endpoint: '+endpoint)
      return subprocess.CompletedProcess(args,0,json.dumps(payload),'')
     return subprocess.CompletedProcess(args,0,'','')
    with patch.object(release,'validate_tag',return_value='0.5.0'),patch.object(release.subprocess,'run',return_value=subprocess.CompletedProcess([],1,'','Not Found')),patch.object(release,'run',side_effect=fake_run):
     release.publish('v0.5.0',path,'owner/repo')
    self.assertEqual(sum(c[:3]==('gh','api','repos/owner/repo/releases') for c in calls),0 if resume else 1)
    self.assertEqual(sum(c[:2]==('gh','api') and c[2].startswith('https://uploads.github.com/') for c in calls),0 if resume else 11)
    self.assertEqual(calls[-1][:5],('gh','api','repos/owner/repo/releases/77','--method','PATCH'))
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
