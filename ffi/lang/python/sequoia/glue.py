from datetime import datetime, timezone

from _sequoia import ffi, lib
from . import error

class SQObject(object):
    # These class attributes determine what features the wrapper class
    # implements.  They must be set to the relevant Sequoia functions.
    #
    # XXX: Once we can assume Python3.6 we can use '__init_subclass__'
    # and reflection on the 'lib' object to set them automatically
    # using the type name.
    _del = None
    _clone = None
    _eq = None
    _str = None
    _hash = None

    def __init__(self, o, context=None, owner=None, references=None):
        if o == ffi.NULL:
            raise error.Error._last(context)
        self.__o = None
        self.ref_replace(o, owner=owner, references=references)
        self.__ctx = context
        if self.__class__._hash is None and not hasattr(self.__class__, '__hash__'):
            # Unhashable types must have '__hash__' set to None.
            # Until we can use '__init_subclass__', we need to patch
            # the class here.  Yuck.
            self.__class__.__hash__ = None

    def ref(self):
        return self.__o

    def ref_consume(self):
        ref = self.ref()
        self._delete(skip_free=True)
        return ref

    def ref_replace(self, new, owner=None, references=None):
        old = self.ref_consume()
        if self._del and owner == None:
            # There is a destructor and We own the referenced object
            # new.
            self.__o = ffi.gc(new, self._del)
        else:
            self.__o = new
        self.__owner = owner
        self.__references = references
        return old

    def _delete(self, skip_free=False):
        if not self.__o:
            return
        if self._del and skip_free:
            ffi.gc(self.__o, None)
        self.__o = None
        self.__owner = None
        self.__references = None

    def context(self):
        return self.__ctx

    def __str__(self):
        if self._str:
            return _str(self._str(self.ref()))
        else:
            return repr(self)

    def __eq__(self, other):
        if self._eq:
            return (isinstance(other, self.__class__)
                    and bool(self._eq(self.ref(), other.ref())))
        else:
            return NotImplemented

    def copy(self):
        if self._clone:
            return self.__class__(self._clone(self.ref()))
        else:
            raise NotImplementedError()

    def __hash__(self):
        return self._hash(self.ref())

def sq_str(s):
    t = ffi.string(s).decode()
    lib.sq_string_free(s)
    return t
_str = sq_str

def sq_iterator(iterator, next_fn, map=lambda x: x):
    while True:
        entry = next_fn(iterator)
        if entry == ffi.NULL:
            break
        yield map(entry)

def sq_time(t):
    if t == 0:
        return None
    else:
        return datetime.fromtimestamp(t, timezone.utc)