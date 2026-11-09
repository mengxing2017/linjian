#!/usr/bin/env python3
"""Validate the full migration chain and source SQL invariants without listening sockets."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
pg_bin = os.environ.get('SOURCE_TEST_PG_BIN')
def executable(name):
    return str(Path(pg_bin) / name) if pg_bin else shutil.which(name)

with tempfile.TemporaryDirectory(prefix='source-access-sql-') as directory:
    subprocess.run([executable('initdb'), '-D', directory, '-A', 'trust', '--no-locale'], check=True, stdout=subprocess.DEVNULL)
    parts = []
    for migration in sorted((root / 'backend/migrations').glob('*.sql')):
        text = migration.read_text()
        if migration.name == '202611080004_configured_wekeeper_credentials.sql':
            upgrade = (root / 'backend/tests/source_configuration_upgrade.sql').read_text()
            parts.append(upgrade.replace('-- APPLY CONFIGURATION MIGRATION HERE', text))
        if migration.name == '202611080005_one_route_per_school.sql':
            upgrade = (root / 'backend/tests/school_route_upgrade.sql').read_text()
            parts.append(upgrade.replace('-- APPLY SCHOOL ROUTE MIGRATION HERE', text))
        parts.append(text)
    sql = '\n'.join(parts)
    repository = (root / 'backend/src/source_access/south/adapter/postgres.rs').read_text()
    upsert = re.search(r'sqlx::query_scalar\("(INSERT INTO school_source_bindings[^"\n]+)"', repository).group(1)
    helper = f"CREATE FUNCTION pg_temp.test_binding(uuid,uuid,uuid,uuid,uuid,text,text,boolean) RETURNS uuid LANGUAGE SQL AS $binding$ {upsert} $binding$;"
    lookup = re.search(r'sqlx::query_scalar\("(SELECT to_jsonb\(c\)[^"\n]+WHERE origin=\$1 AND key_id=\$2[^"\n]+)"', repository).group(1)
    helper += f"\nCREATE FUNCTION pg_temp.configured_connection(text,text) RETURNS jsonb LANGUAGE SQL AS $lookup$ {lookup} $lookup$;"
    smoke = (root / 'backend/tests/source_v2.sql').read_text().replace('BEGIN;', 'BEGIN;\n'+helper, 1)
    sql += '\n' + smoke + '\n' + (root / 'backend/tests/dynamic_source.sql').read_text() + '\n\n'
    result = subprocess.run([executable('postgres'), '--single', '-j', '-D', directory, 'postgres'], input=sql, text=True, capture_output=True)
    errors = [line for line in result.stderr.splitlines() if 'ERROR:' in line or 'FATAL:' in line]
    if result.returncode or errors:
        raise SystemExit('\n'.join(errors) or 'PostgreSQL single-user validation failed')
    print('Full migration chain and source SQL assertions passed.')
