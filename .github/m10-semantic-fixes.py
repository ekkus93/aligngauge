#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/aligngauge-metrics/src/samtools_stats.rs")
text = path.read_text()
old = '''fn fragment_order(flags: u16) -> FragmentOrder {
    if flags & FLAG_PAIRED == 0 || flags & FLAG_READ1 != 0 {
        FragmentOrder::First
    } else if flags & FLAG_READ2 != 0 {
        FragmentOrder::Last
    } else {
        FragmentOrder::Other
    }
}
'''
new = '''fn fragment_order(flags: u16) -> FragmentOrder {
    if flags & FLAG_PAIRED == 0 {
        return FragmentOrder::First;
    }
    match (flags & FLAG_READ1 != 0, flags & FLAG_READ2 != 0) {
        (true, false) => FragmentOrder::First,
        (false, true) => FragmentOrder::Last,
        (false, false) | (true, true) => FragmentOrder::Other,
    }
}
'''
if text.count(old) != 1:
    raise SystemExit(f"fragment_order expected once, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
