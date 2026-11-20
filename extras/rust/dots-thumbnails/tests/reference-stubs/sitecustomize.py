# Python 3.14 defaults multiprocessing to forkserver, which re-imports the
# reference module and loses its module-global factory. The script was written
# for the historical fork default. Restore it for the reference run only.
import multiprocessing

try:
    multiprocessing.set_start_method("fork")
except RuntimeError:
    pass
