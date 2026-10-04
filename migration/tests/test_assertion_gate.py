"""Fault injection proves the contract gate rejects incomplete evidence."""
import copy,importlib.util,json,tempfile,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('assertion_gate',Path(__file__).resolve().parents[1]/'check-assertions.py');gate=importlib.util.module_from_spec(spec);spec.loader.exec_module(gate)
class AssertionGateTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
  self.rust=self.root/'crates/example/tests/contracts.rs';self.rust.parent.mkdir(parents=True);self.rust.write_text('#[test]\nfn contract() {}\n')
  self.python=self.root/'python/tests/test_contract.py';self.python.parent.mkdir(parents=True);self.python.write_text('import unittest\nclass Contract(unittest.TestCase):\n def test_python(self): pass\n')
  self.mapfile=self.root/'migration/assertion-maps/part.json';self.mapfile.parent.mkdir(parents=True)
  self.inventory={'baseline_commit':'baseline','test_count':2,'assertion_call_sites':2,'assertions':[{'id':'old:1:1','file':'old.mjs'},{'id':'old:2:1','file':'old.mjs'}]};(self.root/'migration/legacy-assertions.json').write_text(json.dumps(self.inventory))
  self.mapping={'mappings':[{'ids':['old:1:1'],'checks':[{'file':'crates/example/tests/contracts.rs','test':'contract'}],'note':'original comparison'},{'ids':['old:2:1'],'checks':[{'file':'python/tests/test_contract.py','test':'test_python'}],'note':'original Python behavior'}],'unmapped':[]};self.save()
  self.binary=self.root/'target/debug/deps/contracts-123'
  self.artifact={'reason':'compiler-artifact','profile':{'test':True},'executable':str(self.binary),'target':{'src_path':str(self.rust)}}
  self.rust_log='LAYMESH_CONTRACT_SOURCES '+json.dumps(gate.source_manifest(self.root))+'\n'+json.dumps(self.artifact)+'\n'+json.dumps({'reason':'build-finished','success':True})+f'\nRunning tests/contracts.rs ({self.binary})\nrunning 1 test\ntest contract ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n'
  self.python_log='LAYMESH_CONTRACT_SOURCES '+json.dumps(gate.source_manifest(self.root))+'\n'+'LAYMESH_PYTHON_TEST_SOURCES '+json.dumps({'test_contract.Contract.test_python':str(self.python)})+'\n'+'test_python (test_contract.Contract.test_python) ... ok\n\n----------------------------------------------------------------------\nRan 1 test in 0.001s\n\nOK\n'
 def save(self):
  self.mapfile.write_text(json.dumps(self.mapping))
  # Keep execution snapshots current for structural mapping-failure cases;
  # the explicit stale-source case below deliberately does not refresh them.
  for attr in ('rust_log','python_log'):
   if hasattr(self,attr):setattr(self,attr,'LAYMESH_CONTRACT_SOURCES '+json.dumps(gate.source_manifest(self.root))+'\n'+getattr(self,attr).split('\n',1)[1])
 def result(self,rust=None,python=None):return gate.check(self.root,self.rust_log if rust is None else rust,self.python_log if python is None else python)
 def failed(self,result):self.assertEqual(result['status'],'failed',result);self.assertFalse(result['execution_verified'])
 def test_complete_source_qualified_evidence_passes(self):
  result=self.result();self.assertEqual(result['status'],'passed',result);self.assertTrue(result['execution_verified']);self.assertEqual(result['mapped_sites'],2)
 def test_missing_mapping_fails(self):
  self.mapping['mappings'].pop();self.save();self.failed(self.result())
 def test_missing_or_empty_required_fields_fail(self):
  original=copy.deepcopy(self.mapping)
  for field in ('ids','checks','note'):
   for missing in (True,False):
    with self.subTest(field=field,missing=missing):
     self.mapping=copy.deepcopy(original)
     if missing:del self.mapping['mappings'][0][field]
     else:self.mapping['mappings'][0][field]='' if field=='note' else []
     self.save();self.failed(self.result())
  for field in ('file','test'):
   self.mapping=copy.deepcopy(original);del self.mapping['mappings'][0]['checks'][0][field];self.save();self.failed(self.result())
 def test_missing_test_file_or_test_declaration_fails(self):
  self.rust.unlink();self.failed(self.result());self.rust.write_text('fn contract() {}\n');self.failed(self.result());self.rust.write_text('#[test]\nfn different() {}\n');self.failed(self.result())
 def test_missing_python_execution_fails(self):
  self.failed(gate.check(self.root,self.rust_log));self.failed(self.result(python=''))
 def test_truncated_rust_and_python_logs_fail(self):
  self.failed(self.result(rust=self.rust_log.split('test result:')[0]));self.failed(self.result(python=self.python_log.split('\nOK')[0]));self.failed(self.result(rust='test contract ... ok\n'))
 def test_same_name_in_another_source_cannot_substitute(self):
  artifact=copy.deepcopy(self.artifact);artifact['target']['src_path']=str(self.root/'crates/other/tests/contracts.rs');bad=self.rust_log.replace(json.dumps(self.artifact),json.dumps(artifact));self.failed(self.result(rust=bad));self.failed(self.result(python=self.python_log.replace('test_contract.Contract','other_file.Contract')))
 def test_same_python_method_in_another_class_cannot_substitute(self):
  self.failed(self.result(python=self.python_log.replace('test_contract.Contract','test_contract.OtherClass')))
 def test_ignored_or_failed_tests_are_not_execution(self):
  self.failed(self.result(rust=self.rust_log.replace('test contract ... ok','test contract ... ignored')));self.failed(self.result(python=self.python_log.replace('... ok',"... skipped 'not run'")));self.failed(self.result(rust=self.rust_log.replace('test result: ok','test result: FAILED')))
 def test_unknown_ids_and_unresolved_entries_fail(self):
  self.mapping['mappings'][0]['ids'].append('unknown:1:1');self.save();self.failed(self.result());self.mapping['mappings'][0]['ids'].pop();self.mapping['unmapped']=['old:1:1'];self.save();self.failed(self.result())
 def test_duplicate_inventory_ids_are_not_silently_collapsed(self):
  self.inventory['assertions'].append(self.inventory['assertions'][0]);(self.root/'migration/legacy-assertions.json').write_text(json.dumps(self.inventory));self.failed(self.result())
 def test_missing_test_binary_termination_fails(self):
  artifact=copy.deepcopy(self.artifact);artifact['executable']=str(self.binary)+'other';self.failed(self.result(rust=json.dumps(artifact)+'\n'+self.rust_log))

 def test_changed_source_invalidates_old_passing_log(self):
  self.rust.write_text('#[test]\nfn contract() { assert_eq!(1,1); }\n');self.failed(self.result())

 def test_missing_outcome_inside_complete_summary_fails(self):
  self.failed(self.result(rust=self.rust_log.replace('test contract ... ok\n','')))
  self.failed(self.result(python=self.python_log.replace('Ran 1 test','Ran 2 tests')))
 def test_warnings_interleaved_with_python_result_preserve_identity(self):
  log=self.python_log.replace('... ok','... some.py:12: UserWarning: expected warning\n  warn()\nok')
  self.assertEqual(self.result(python=log)['status'],'passed')
 def test_duplicate_execution_snapshots_and_outcomes_fail(self):
  self.failed(self.result(rust=self.rust_log.split('\n',1)[0]+'\n'+self.rust_log))
  self.failed(self.result(rust=self.rust_log.replace('test contract ... ok','test contract ... ok\ntest contract ... ok')))
  self.failed(self.result(python=self.python_log.replace('test_python (test_contract.Contract.test_python) ... ok','test_python (test_contract.Contract.test_python) ... ok\ntest_python (test_contract.Contract.test_python) ... ok')))

 def test_changed_fixture_invalidates_old_passing_log(self):
  fixture=self.root/'tests/fixtures/contour.json';fixture.parent.mkdir(parents=True);fixture.write_text('{"alpha":0.5}')
  self.save();self.assertEqual(self.result()['status'],'passed')
  fixture.write_text('{"alpha":1}');self.failed(self.result())

 def test_same_python_module_class_method_from_another_file_fails(self):
  self.failed(self.result(python=self.python_log.replace(json.dumps(str(self.python)),json.dumps(str(self.root/'another/tests/test_contract.py')))))
 def test_missing_python_file_identity_fails(self):
  self.failed(self.result(python='\n'.join(line for line in self.python_log.splitlines() if not line.startswith('LAYMESH_PYTHON_TEST_SOURCES '))))
