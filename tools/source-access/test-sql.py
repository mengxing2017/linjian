#!/usr/bin/env python3
"""Validate current source schema through the shared isolated migration checker."""
from pathlib import Path
import runpy

runpy.run_path(str(Path(__file__).resolve().parents[1] / 'test_migrations.py'), run_name='__main__')
