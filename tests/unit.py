"""Unit tests for the cmsis_pack_manager module"""

from os.path import join
from string import ascii_lowercase, ascii_letters, hexdigits
from unittest.mock import patch, MagicMock, call
from hypothesis import given, settings, example
from hypothesis.strategies import booleans, text, lists, just, integers, tuples
from hypothesis.strategies import dictionaries, fixed_dictionaries
from jinja2 import Template

import cmsis_pack_manager

@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase, min_size=1),
       text(alphabet=ascii_lowercase, min_size=1),
       text(alphabet=ascii_lowercase, min_size=1))
def test_pdsc_from_cache(data_path, vendor, pack, version):
    @patch("cmsis_pack_manager.open", create=True)
    def inner_test(_open):
        _open.return_value.__enter__.return_value = MagicMock
        c = cmsis_pack_manager.Cache(True, True, data_path=data_path)
        device = {'from_pack': {'vendor': vendor , 'pack': pack,
                                'version': version}}
        c.pdsc_from_cache(device)
        assert(vendor in _open.call_args[0][0])
        assert(pack in _open.call_args[0][0])
        assert(version in _open.call_args[0][0])
    inner_test()

@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase, min_size=1),
       text(alphabet=ascii_lowercase, min_size=1),
       text(alphabet=ascii_lowercase, min_size=1))
def test_pack_from_cache(data_path, vendor, pack, version):
    @patch("cmsis_pack_manager.ZipFile")
    def inner_test(_zf):
        c = cmsis_pack_manager.Cache(True, True, data_path=data_path)
        device = {'from_pack': {'vendor': vendor , 'pack': pack,
                                'version': version}}
        c.pack_from_cache(device)
        assert(vendor in _zf.call_args[0][0])
        assert(pack in _zf.call_args[0][0])
        assert(version in _zf.call_args[0][0])
    inner_test()
