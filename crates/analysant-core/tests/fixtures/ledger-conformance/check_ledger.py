# SPDX-License-Identifier: MIT OR Apache-2.0
# Independent check of rules 5-6 in docs/LEDGER-WRITER-CONFORMANCE.md: recomputes every event id and text_hash
# of a ledger file from its decoded fields. Usage: python -I check_ledger.py FILE.jsonl
import json, struct, sys

M = 2**64 - 1
def f(parts):
    h = 0xcbf29ce484222325
    for p in parts:
        for b in p:
            h = ((h ^ b) * 0x100000001b3) & M
        h = ((h ^ 0xff) * 0x100000001b3) & M
    return h

KIND = {"document": 0, "section": 1, "block": 2, "claim": 3}
RANK = {"minted": 0, "revised": 1, "moved": 2, "gone": 3}

def th(text):
    return "%016x" % f([text.encode("utf-8")])

ok = True
raw = open(sys.argv[1], "rb").read()
assert b"\r" not in raw.replace(b"\\r", b""), "CR in file"
for i, line in enumerate(raw.decode("utf-8").split("\n")):
    if not line:
        continue
    r = json.loads(line)
    if r["type"] == "format":
        continue
    parts = [bytes([RANK[r["type"]]]), r["unit"].encode(), struct.pack("<I", r["revision"]), r["document"].encode()]
    if r["type"] == "minted":
        parts += [bytes([KIND[r["kind"]]]), "\x1f".join(r["section_path"]).encode(), r["text_hash"].encode()]
    elif r["type"] == "revised":
        parts += [r["text_hash"].encode()]
    elif r["type"] == "moved":
        parts += ["\x1f".join(r["section_path"]).encode()]
    eid = "%016x%016x" % (f([b"hi"] + parts), f(parts))
    good = eid == r["id"] and (("text" not in r) or th(r["text"]) == r["text_hash"])
    ok &= good
    print(i + 1, r["type"], "OK" if good else "MISMATCH")
print("ALL OK" if ok else "FAIL")
