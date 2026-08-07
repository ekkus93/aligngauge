#!/usr/bin/env python3
from pathlib import Path

path = Path('.github/workflows/ci.yml')
text = path.read_text()
old_bash = '''          bash -n tools/reference/samtools/run-target-depth.sh
          bash -n testdata/hg002/prepare.sh
'''
new_bash = '''          bash -n tools/reference/samtools/run-target-depth.sh
          bash -n tools/reference/samtools/run-stats.sh
          bash -n tools/reference/multiqc/validate-samtools-stats.sh
          bash -n testdata/hg002/prepare.sh
'''
if text.count(old_bash) != 1:
    raise SystemExit(f'bash validation anchor count={text.count(old_bash)}')
text = text.replace(old_bash, new_bash, 1)
old_python = '''              Path("tools/reference/samtools/compare-target-depth.py"),
              Path("tools/ci/check_coverage_rss.py"),
'''
new_python = '''              Path("tools/reference/samtools/compare-target-depth.py"),
              Path("tools/reference/samtools/compare-stats.py"),
              Path("tools/ci/check_coverage_rss.py"),
'''
if text.count(old_python) != 1:
    raise SystemExit(f'python validation anchor count={text.count(old_python)}')
path.write_text(text.replace(old_python, new_python, 1))
