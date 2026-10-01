import py_compile
import sys


if len(sys.argv) != 3:
    raise SystemExit("usage: python_compile.py SOURCE OUTPUT")

py_compile.compile(sys.argv[1], cfile=sys.argv[2], doraise=True)
