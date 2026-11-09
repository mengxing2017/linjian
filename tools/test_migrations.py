#!/usr/bin/env python3
"""Validate the real migrations with PostgreSQL single-user mode (no sockets)."""
import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', default=os.environ.get('LINJIAN_POSTGRES_BIN'))
    args = parser.parse_args()
    if not args.bin_dir:
        parser.error('set LINJIAN_POSTGRES_BIN or --bin-dir')
    pg = Path(args.bin_dir).resolve()
    files = sorted(Path('backend/migrations').glob('*.sql'))
    sql = '\n'.join(p.read_text() for p in files)
    sql += Path('backend/tests/forum_migrations.sql').read_text()
    sql += Path('backend/tests/dynamic_source.sql').read_text()
    # -j terminates an input unit on a blank line. Send one SQL statement per
    # unit, preserving dollar-quoted function bodies and explicit transactions.
    sql = re.sub(r'\n\s*\n', '\n', sql)
    tokens = re.findall(r"\$\$[\s\S]*?\$\$|'(?:''|[^'])*'|--[^\n]*|;|[^;$'\-]+|.", sql)
    sql = ''.join(';\n\n' if token == ';' else token for token in tokens)

    with tempfile.TemporaryDirectory(prefix='linjian-sql-') as directory:
        data = Path(directory) / 'data'
        subprocess.run([str(pg / 'initdb'), '-D', str(data), '-U', 'postgres', '--auth-local=trust', '--auth-host=reject', '--no-locale', '--encoding=UTF8'], check=True, stdout=subprocess.DEVNULL)
        run = subprocess.run([str(pg / 'postgres'), '--single', '-j', '-D', str(data), 'postgres'], input=sql, text=True, capture_output=True)
        if run.returncode or re.search(r'\b(ERROR|FATAL|PANIC):', run.stdout + run.stderr):
            print(run.stdout)
            print(run.stderr)
            raise SystemExit(1)
    print(f'PostgreSQL 17: {len(files)} migrations and forum/outbox transaction assertions passed.')


if __name__ == '__main__':
    main()
