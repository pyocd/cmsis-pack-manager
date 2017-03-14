"""Unit tests for the ArmPackManager module"""

from os.path import join
from string import ascii_lowercase
from mock import patch, MagicMock
from hypothesis import given, settings, example
from hypothesis.strategies import booleans, text, lists, just, integers

import ArmPackManager

@given(booleans(), booleans(), text(), text())
@example(True, True, '', '')
@example(True, True, '/', '/')
def test_init(silent, no_timeouts, json_path, data_path):
    obj = ArmPackManager.Cache(silent, no_timeouts, json_path=json_path, data_path=data_path)
    assert(obj.index_path)
    assert(obj.aliases_path)
    assert(obj.data_path)

@given(lists(text(min_size=1), min_size=1), just(None))
@example(["1.0.0", "0.1.0", "0.0.1"], "1.0.0")
@example(["1.0.0", "19.0.0", "2.0.0"], "19.0.0")
def test_largest_version(version_strings, max_version):
    newest = ArmPackManager.largest_version(version_strings)
    if max_version:
        assert(newest == max_version)

@given(lists(integers()))
def test_do_queue(queue):
    to_run = MagicMock()
    ArmPackManager.do_queue(ArmPackManager.Reader, to_run, queue)
    for blah in queue:
        to_run.assert_any_call(blah)

@given(text(alphabet=ascii_lowercase), text(alphabet=ascii_lowercase + ":/_."))
@example("http", "google.com")
def test_strip_protocol(protocol, url):
    uri = protocol + "://" + url
    assert(ArmPackManager.strip_protocol(uri) == url)

@given(text(alphabet=ascii_lowercase + ":/_."), text())
def test_cache_file(url, contents):
    @patch("ArmPackManager.Cache.display_counter")
    @patch("ArmPackManager.urlopen")
    @patch("ArmPackManager.open", create=True)
    def inner_test(_open, _urlopen, _):
        _open.return_value = MagicMock(spec=file)
        _urlopen.return_value.read.return_value = contents
        c = ArmPackManager.Cache(True, True)
        c.cache_file(url)
        _urlopen.assert_called_with(url)
        _open.assert_called()
        _open.return_value.__enter__.return_value.write.assert_called_with(contents)
    inner_test()


@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase +"/", min_size=1))
def test_pdsc_from_cache(data_path, url):
    @patch("ArmPackManager.BeautifulSoup")
    @patch("ArmPackManager.open", create=True)
    def inner_test(_open, _bs):
        _open.return_value.__enter__.return_value = MagicMock
        c = ArmPackManager.Cache(True, True, data_path=data_path)
        c.pdsc_from_cache(url)
        _open.called_with(join(data_path, url), "r")
        _bs.called_with(_open.return_value.__enter__.return_value, "html.parser")
    inner_test()

@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase +"/", min_size=1))
def test_pack_from_cache(data_path, url):
    @patch("ArmPackManager.ZipFile")
    def inner_test(_zf):
        c = ArmPackManager.Cache(True, True, data_path=data_path)
        device = {'pack_file': url}
        c.pack_from_cache(device)
        _zf.called_with(join(data_path, url))
    inner_test()
