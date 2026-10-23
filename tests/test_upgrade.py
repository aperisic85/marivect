#!/usr/bin/env python3
import pathlib, subprocess, tempfile, unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
class Upgrade(unittest.TestCase):
    def test_preserves_secrets_and_applied_migration_and_backs_up_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            target=pathlib.Path(tmp)/'existing';target.mkdir()
            (target/'compose.yaml').write_text('old compose\n')
            migration=target/'backend/migrations/0001_foundation.sql';migration.parent.mkdir(parents=True);migration.write_text('-- exact user-applied migration bytes\n')
            (target/'.env').write_text('keep-this-private-value\n')
            result=subprocess.run(['python3',str(ROOT/'scripts/install-upgrade.py'),'--target',str(target)],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(migration.read_text(),'-- exact user-applied migration bytes\n')
            self.assertEqual((target/'.env').read_text(),'keep-this-private-value\n')
            self.assertTrue((target/'backend/migrations/0002_identity.sql').is_file())
            copies=list((target/'.marivect-backups').glob('*/compose.yaml'))
            self.assertEqual(len(copies),1);self.assertEqual(copies[0].read_text(),'old compose\n')
if __name__=='__main__':unittest.main()
