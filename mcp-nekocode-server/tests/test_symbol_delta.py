"""End-to-end comparison invariants using actual core packets and final MCP output."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from test_contract_schemas import assert_standard_json_schema, jsonschema

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / 'nekocode-workspace'
FAKE = WORKSPACE / 'nekocode-core/src/symbol_context/fixtures/fake-ra.py'
DEFINITION = 'pub fn target(x: i32) -> i32 {\n    x / 2\n}\n'
KEPT = 'pub fn kept() -> i32 {\n    let _url = "https://example.com/api";\n    target(4)\n}\n'
REMOVED = 'pub fn removed() -> i32 { target(8) }\n'
ADDED = 'pub fn added() -> i32 { target(12) }\n'


@unittest.skipIf(jsonschema is None, 'install requirements-dev.txt')
class SymbolDeltaTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(['cargo','build','--locked','--offline','-q','-p','nekocode'],
                       cwd=WORKSPACE,check=True,capture_output=True)
        cls.binary = WORKSPACE / 'target/debug/nekocode'
        cls.schema = json.loads((ROOT / 'schemas/symbol-delta-v1.schema.json').read_text())
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        spec = importlib.util.spec_from_file_location('delta_gateway', ROOT / 'mcp-nekocode-server/mcp_server_rust_first.py')
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix='nekocode-delta-')
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / 'src').mkdir()
        (self.root / 'Cargo.toml').write_text('[package]\nname="delta_fixture"\nversion="0.1.0"\nedition="2021"\n')
        self.source = self.root / 'src/lib.rs'
        self.fake = self.root / 'fake-ra'
        self.fake.write_bytes(FAKE.read_bytes()); self.fake.chmod(0o755)
        self.env = dict(os.environ, NEKOCODE_RUST_ANALYZER_PATH=str(self.fake))
        self.offline = dict(os.environ, PATH='/no-executables', NEKOCODE_RUST_ANALYZER_PATH='/no-analyzer')
        self.before = self.capture('before.json', DEFINITION + KEPT + REMOVED)
        self.after = self.capture('after.json', '// unrelated inserted line\n\n' + DEFINITION + KEPT + ADDED)

    def capture(self, name, source):
        self.source.write_text(source)
        path = self.root / name
        subprocess.run([str(self.binary),'context',str(self.root),'--symbol','target','--max-items','1',
                        '--save-packet',str(path)],env=self.env,check=True,capture_output=True)
        return path

    def compare(self, *extra, before=None, after=None):
        run = subprocess.run([str(self.binary),'context','--packet',str(after or self.after),
                              '--compare-packet',str(before or self.before),*extra],
                             env=self.offline,check=True,text=True,capture_output=True)
        result = json.loads(run.stdout)
        assert_standard_json_schema(result,self.schema)
        self.assertEqual(result['budget']['serialized_bytes'], len(run.stdout.rstrip('\n').encode()))
        return result

    def mutated(self, update):
        value = json.loads(self.after.read_text())
        update(value)
        value['integrity_sha256'] = ''
        raw = json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()
        value['integrity_sha256'] = 'sha256:' + hashlib.sha256(raw).hexdigest()
        path = self.root / 'mutated.json'
        path.write_text(json.dumps(value))
        return path

    def test_inserted_lines_added_removed_and_full_capture_not_first_page(self):
        result = self.compare()
        self.assertEqual(result['comparison_status'],'observed_comparable')
        self.assertEqual([result['totals'][k] for k in ['added','removed','matched','unresolved']],[1,1,1,0])
        matched = next(c for c in result['changes'] if c['change']=='matched')
        self.assertNotEqual(matched['before']['location']['start']['line'],matched['after']['location']['start']['line'])
        self.assertEqual(matched['before']['source_line'], matched['after']['source_line'])
        # Changing/deleting current disk is immaterial for historical comparison.
        self.source.unlink()
        self.assertEqual(result,self.compare())

    def test_paging_cursor_binding_and_original_packet_expansion(self):
        first = self.compare('--max-items','1')
        second = self.compare('--max-items','1','--cursor',first['next_cursor'])
        self.assertNotEqual(first['changes'],second['changes'])
        self.assertEqual(first['totals']['added'],second['totals']['added'])
        bad = subprocess.run([str(self.binary),'context','--packet',str(self.before),'--compare-packet',str(self.after),
                              '--cursor',first['next_cursor']],env=self.offline,capture_output=True)
        self.assertNotEqual(bad.returncode,0)
        removed = second['changes'][0]['before']
        expanded = json.loads(subprocess.run([str(self.binary),'context','--packet',str(self.before),
                              '--item',removed['item_id']],env=self.offline,capture_output=True,check=True,text=True).stdout)
        self.assertIn('target(8)',expanded['items'][0]['code'])

    def test_budget_counts_and_no_empty_loop(self):
        expected = self.compare()['totals']
        for budget in [1,500,1000,1500,2000,8000]:
            with self.subTest(budget=budget):
                result = self.compare('--budget',str(budget))
                self.assertEqual({k:result['totals'][k] for k in ['added','removed','matched','unresolved']},
                                 {k:expected[k] for k in ['added','removed','matched','unresolved']})
                if not result['changes']:
                    self.assertEqual(result['status'],'output_limited')
                    self.assertIsNone(result['next_cursor'])
                if not result['budget']['exceeded']:
                    self.assertLessEqual(result['budget']['serialized_bytes'],budget*4)

    def test_edited_reference_in_same_container_is_unresolved(self):
        after = self.capture('edit.json',DEFINITION + KEPT.replace('target(4)','target(6)') + REMOVED)
        result = self.compare(after=after)
        self.assertEqual([result['totals'][k] for k in ['added','removed','matched','unresolved']],[0,0,1,2])
        self.assertEqual(result['status'],'partial')

    def test_target_body_change_and_unicode_anchors(self):
        unicode_caller = 'pub fn 猫() -> i32 { let _s = "🐈https://example.com/a"; target(4) }\n'
        before = self.capture('unicode-before.json', DEFINITION + unicode_caller)
        after = self.capture('unicode-after.json', '\n' + DEFINITION.replace('x / 2','x / 3') + unicode_caller)
        result = self.compare(before=before,after=after)
        self.assertEqual(result['totals']['matched'],1)
        self.assertIn('🐈https://example.com/a',result['changes'][0]['after']['source_line'])

    def test_changed_caller_declaration_is_unresolved(self):
        before = self.capture('caller-before.json', DEFINITION + 'pub fn caller() -> i32 { target(4) }\n')
        after = self.capture('caller-after.json', DEFINITION + 'pub fn caller() -> i32 { target(6) }\n')
        result = self.compare(before=before,after=after)
        self.assertEqual(result['totals']['unresolved'],2)
        self.assertEqual(result['totals']['removed'],0)

    def test_duplicate_anchors_are_not_arbitrarily_matched(self):
        duplicated = DEFINITION + 'pub fn repeated() {\n    target(4);\n    target(4);\n}\n'
        before = self.capture('duplicates-before.json',duplicated)
        after = self.capture('duplicates-after.json','\n'+duplicated)
        result = self.compare(before=before,after=after)
        self.assertEqual(result['totals']['unresolved'],4)
        self.assertEqual(result['totals']['matched'],0)

    def test_failures_scope_config_backend_and_target_are_not_removed(self):
        def config(value):
            value['inputs']['files']['Cargo.toml'] = 'sha256:changed'
        def failed(value):
            q = next(q for q in value['response']['queries'] if q['method']=='textDocument/references')
            q.update(status='timed_out',result_count=None)
        cases = [
            (lambda v:v['response']['scope'].update(features='all_features'),'scope_mismatch'),
            (lambda v:v['response']['backend'].update(version='other'),'backend_version_mismatch_or_unknown'),
            (lambda v:v['response']['freshness'].update(state='changed_during_observation'),'after_capture_not_stable'),
            (lambda v:v['response'].update(status='partial'),'after_capture_incomplete'),
            (lambda v:v['response']['target'].update(resolution='not_found'),'target_identity_changed_or_unresolved'),
            (config,'configuration_inputs_changed'),(failed,'after_references_not_fully_captured'),
        ]
        for update,reason in cases:
            with self.subTest(reason=reason):
                result = self.compare(after=self.mutated(update))
                self.assertEqual(result['comparison_status'],'not_comparable')
                self.assertIn(reason,result['reasons'])
                self.assertIsNone(result['totals']['removed'])
                self.assertEqual(result['changes'],[])

    def test_zero_references_is_a_comparable_observation(self):
        after = self.capture('zero.json',DEFINITION)
        result = self.compare(after=after)
        self.assertEqual(result['totals']['removed'],2)
        self.assertEqual(result['after']['reference_query']['result_count'],0)
        result = self.compare(before=after,after=after)
        self.assertEqual(result['totals']['removed'],0)
        self.assertEqual(result['status'],'completed')

    def test_tampering_and_cli_option_conflicts_are_rejected(self):
        bad = self.root/'tampered.json'
        value = json.loads(self.after.read_text());value['response']['status']='partial'
        bad.write_text(json.dumps(value))
        for args in [
            ['--packet',str(bad),'--compare-packet',str(self.before)],
            ['--compare-packet',str(self.before)],
            ['--packet',str(self.after),'--compare-packet',str(self.before),'--item','x'],
            ['--packet',str(self.after),'--compare-packet',str(self.before),'--all-features'],
        ]:
            with self.subTest(args=args):
                self.assertNotEqual(subprocess.run([str(self.binary),'context',*args],env=self.offline,capture_output=True).returncode,0)

    def test_final_mcp_and_summary_preserve_evidence_and_reasons(self):
        server = self.module.RustFirstMCPServer(binary_path=self.binary)
        with patch.dict(os.environ,dict(self.offline,NEKOCODE_CLI_CWD=str(self.root))):
            result = server.handle_tool_call({'name':'nekocode_context','arguments':{
                'packet':self.after.name,'compare_packet':self.before.name}})
        self.assertFalse(result['isError'],result)
        assert_standard_json_schema(result['structuredContent'],self.schema)
        self.assertEqual(result['structuredContent'],json.loads(result['content'][0]['text']))
        self.assertEqual(result['structuredContent'],self.compare())
        for arguments in [{'compare_packet':str(self.before)}, {'symbol':'target','compare_packet':str(self.before)},
                          {'packet':str(self.after),'compare_packet':str(self.before),'item':'x'}]:
            self.assertTrue(server.handle_tool_call({'name':'nekocode_context','arguments':arguments})['isError'])
        summary = subprocess.run([str(self.binary),'context','--packet',str(self.after),'--compare-packet',str(self.before),'--format','summary'],
                                 env=self.offline,capture_output=True,text=True,check=True).stdout
        self.assertIn('Added: 1; removed: 1; matched: 1',summary)
        self.assertIn('target(12)',summary)


if __name__ == '__main__':
    unittest.main()
