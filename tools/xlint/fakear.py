import sys
for x in sys.argv[1:]:
    if x.endswith(".a"):
        open(x, "wb").close()
sys.exit(0)
