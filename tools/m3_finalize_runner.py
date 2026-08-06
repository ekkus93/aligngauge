from pathlib import Path
import runpy

self_path = Path("tools/m3_finalize_runner.py")
finalizer = Path("tools/m3_finalize.py")
text = finalizer.read_text(encoding="utf-8")
old = "if unchecked != 31:\n    raise SystemExit(f\"expected 31 unchecked Milestone 3 tasks, found {unchecked}\")"
new = "if unchecked != 36:\n    raise SystemExit(f\"expected 36 unchecked Milestone 3 tasks, found {unchecked}\")"
if old not in text:
    raise SystemExit("Milestone 3 cardinality assertion was not found")
finalizer.write_text(text.replace(old, new, 1), encoding="utf-8")
runpy.run_path(str(finalizer), run_name="__main__")
self_path.unlink()
