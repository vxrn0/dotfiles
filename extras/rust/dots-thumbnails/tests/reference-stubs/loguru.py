# Minimal stand-in for loguru. The reference script imports it at module load,
# but the package is not installed here. Only the logger surface that
# thumbgen.py uses is implemented; it intentionally prints nothing.
class _Logger:
    def remove(self, *args, **kwargs):
        pass

    def add(self, *args, **kwargs):
        pass

    def debug(self, *args, **kwargs):
        pass

    def info(self, *args, **kwargs):
        pass

    def catch(self, *args, **kwargs):
        def decorate(function):
            return function

        return decorate


logger = _Logger()
