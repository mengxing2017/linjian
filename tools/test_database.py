#!/usr/bin/env python3
"""Run forum integration tests against a temporary Unix-socket-only PostgreSQL."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', default=os.environ.get('LINJIAN_POSTGRES_BIN'))
    args = parser.parse_args()
    if not args.bin_dir:
        parser.error('set LINJIAN_POSTGRES_BIN or --bin-dir to PostgreSQL 17 bin directory')
    pg = Path(args.bin_dir).resolve()
    with tempfile.TemporaryDirectory(prefix='linjian-test-') as directory:
        root = Path(directory)
        data = root / 'data'
        subprocess.run([str(pg / 'initdb'), '-D', str(data), '-U', 'postgres', '--auth-local=trust', '--auth-host=reject', '--no-locale', '--encoding=UTF8'], check=True, stdout=subprocess.DEVNULL)
        started = False
        try:
            subprocess.run([str(pg / 'pg_ctl'), '-D', str(data), '-l', str(root / 'postgres.log'), '-o', f"-c listen_addresses='' -k {root}", '-w', 'start'], check=True)
            started = True
            env = os.environ.copy()
            env['LINJIAN_TEST_DATABASE_URL'] = f'postgresql://postgres@localhost/postgres?host={root}'
            subprocess.run(['cargo', 'test', '--locked', '--manifest-path', 'backend/Cargo.toml', '--test', 'forum_database', '--', '--ignored'], env=env, check=True)
        finally:
            if started:
                subprocess.run([str(pg / 'pg_ctl'), '-D', str(data), '-m', 'immediate', '-w', 'stop'], check=True)
            else:
                log = root / 'postgres.log'
                if log.exists():
                    print(log.read_text())


if __name__ == '__main__':
    main()
