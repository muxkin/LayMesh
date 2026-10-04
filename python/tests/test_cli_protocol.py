"""Observable CLI contracts: usage failures never create or replace output files."""
import json,subprocess,tempfile,unittest
from pathlib import Path
from laymesh.bridge import _command

class CliProtocol(unittest.TestCase):
 def test_invalid_dpi_is_usage_and_preserves_destination(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);source=root/'main.lay';source.write_text('page=canvas(size=(10,10))');dest=root/'out.png';dest.write_bytes(b'existing-output')
   for value in (None,'not-a-number','NaN','inf','0','-1'):
    with self.subTest(dpi=value):
     args=['render',str(source),'-o',str(dest),'--dpi'];args+=[] if value is None else [value]
     result=subprocess.run([*_command(),*args],capture_output=True,text=True);self.assertEqual(result.returncode,2);self.assertIn('用法',result.stderr);self.assertEqual(dest.read_bytes(),b'existing-output');self.assertFalse(list(root.glob('*.tmp')))
 def test_inspection_json_and_warning_precedence(self):
  with tempfile.TemporaryDirectory() as tmp:
   source=Path(tmp)/'main.lay';source.write_text('page=canvas(size=(40,30))\npage.add(text("missing",font_family="missing-font-that-cannot-exist"))')
   result=subprocess.run([*_command(),'inspect',str(source),'--json','--warnings','hide'],capture_output=True,text=True);self.assertEqual(result.returncode,0,result.stderr);value=json.loads(result.stdout);self.assertEqual(set(value),{'schema_version','units','page','plots','warnings'});self.assertEqual(value['schema_version'],8);self.assertEqual(value['units'],'mm');self.assertEqual(value['page'],{'width':40.0,'height':30.0,'unit':'mm','layout_dpi':96.0});self.assertTrue(any(w['code']=='W_FONT' for w in value['warnings']));self.assertEqual(result.stderr,'')
 def test_warning_switches_and_env_precedence(self):
  import os
  with tempfile.TemporaryDirectory() as tmp:
   source=Path(tmp)/'main.lay';source.write_text('page=canvas(size=(130,120))\np=plot(projection="polar",size=(100,90),r=axis(range=(0,10)))\np.line(theta=[0,90,180,270],r=[-10,10,10,10])\npage.add(p)')
   def run(args,mode):return subprocess.run([*_command(),*args],capture_output=True,text=True,env={**os.environ,'LAYMESH_WARNINGS':mode})
   hidden=run(['validate',str(source)],'hide');self.assertEqual(hidden.returncode,0,hidden.stderr);self.assertEqual(hidden.stderr,'')
   shown=run(['validate',str(source),'--warnings','show'],'hide');self.assertEqual(shown.returncode,0,shown.stderr);self.assertIn('W_POLAR_NEGATIVE_RADIUS',shown.stderr)
   override=run(['validate',str(source),'--warnings','hide'],'show');self.assertEqual(override.stderr,'')
   inspection=run(['inspect',str(source),'--json','--warnings','hide'],'hide');self.assertEqual(inspection.returncode,0,inspection.stderr);self.assertTrue(json.loads(inspection.stdout)['warnings'])
   source.write_text('invalid()');invalid=run(['validate',str(source)],'hide');self.assertNotEqual(invalid.returncode,0);self.assertTrue(invalid.stderr)
