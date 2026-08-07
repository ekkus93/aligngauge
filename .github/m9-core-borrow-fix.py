#!/usr/bin/env python3
"""Move targeted finalization before legacy per-reference state is consumed."""

from pathlib import Path

path = Path("crates/aligngauge-coverage/src/accumulator/reduce.rs")
text = path.read_text()
old = "        let per_reference = self\n            .references\n            .into_iter()"
new = "        let targeted = self.finish_targeted()?;\n        let per_reference = self\n            .references\n            .into_iter()"
if text.count(old) != 1:
    raise SystemExit(f"expected one per-reference finalization marker, found {text.count(old)}")
text = text.replace(old, new, 1)
old = "\n        let targeted = self.finish_targeted()?;\n\n        Ok(CoverageReport {"
new = "\n        Ok(CoverageReport {"
if text.count(old) != 1:
    raise SystemExit(f"expected one late targeted finalization marker, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
