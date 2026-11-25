import sys
a = sys.argv[1:]
out = None
if "-o" in a:
    out = a[a.index("-o") + 1]
for x in a:
    if x.startswith("-o") and len(x) > 2:
        out = x[2:]
if out:
    open(out, "wb").close()
sys.exit(0)
