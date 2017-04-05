"""Unit tests for the cmsis_pack_manager module"""

from os.path import join
from string import ascii_lowercase
from mock import patch, MagicMock, call
from hypothesis import given, settings, example
from hypothesis.strategies import booleans, text, lists, just, integers, tuples
from jinja2 import Template
from bs4 import BeautifulSoup

import cmsis_pack_manager

@given(booleans(), booleans(), text(), text())
@example(True, True, '', '')
@example(True, True, '/', '/')
def test_init(silent, no_timeouts, json_path, data_path):
    obj = cmsis_pack_manager.Cache(silent, no_timeouts, json_path=json_path, data_path=data_path)
    assert(obj.index_path)
    assert(obj.aliases_path)
    assert(obj.data_path)

@given(lists(text(min_size=1), min_size=1), just(None))
@example(["1.0.0", "0.1.0", "0.0.1"], "1.0.0")
@example(["1.0.0", "19.0.0", "2.0.0"], "19.0.0")
def test_largest_version(version_strings, max_version):
    newest = cmsis_pack_manager.largest_version(version_strings)
    if max_version:
        assert(newest == max_version)

@given(lists(integers()))
def test_do_queue(queue):
    to_run = MagicMock()
    cmsis_pack_manager.do_queue(cmsis_pack_manager.Reader, to_run, queue)
    for blah in queue:
        to_run.assert_any_call(blah)

@given(text(alphabet=ascii_lowercase), text(alphabet=ascii_lowercase + ":/_."))
@example("http", "google.com")
@example("http", "google.com://foo")
def test_strip_protocol(protocol, url):
    uri = protocol + "://" + url
    assert(cmsis_pack_manager.strip_protocol(uri) == url)

@given(text(alphabet=ascii_lowercase + ":/_."), text())
def test_cache_file(url, contents):
    @patch("cmsis_pack_manager.Cache._cache_lookup")
    @patch("cmsis_pack_manager.Cache.display_counter")
    @patch("cmsis_pack_manager.urlopen")
    @patch("cmsis_pack_manager.makedirs")
    @patch("cmsis_pack_manager.open", create=True)
    def inner_test(_open, _, _urlopen, __, _cache_lookup):
        _open.return_value = MagicMock(spec=file)
        _urlopen.return_value.read.return_value = contents
        c = cmsis_pack_manager.Cache(True, True)
        c.cache_file(url)
        _urlopen.assert_called_with(url)
        _open.assert_called_with(_cache_lookup.return_value, "wb+")
        _open.return_value.__enter__.return_value.write.assert_called_with(contents)
    inner_test()


@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase +"/", min_size=1))
def test_pdsc_from_cache(data_path, url):
    @patch("cmsis_pack_manager.BeautifulSoup")
    @patch("cmsis_pack_manager.open", create=True)
    def inner_test(_open, _bs):
        _open.return_value.__enter__.return_value = MagicMock
        c = cmsis_pack_manager.Cache(True, True, data_path=data_path)
        c.pdsc_from_cache(url)
        _open.called_with(join(data_path, url), "r")
        _bs.called_with(_open.return_value.__enter__.return_value, "html.parser")
    inner_test()

@given(text(alphabet=ascii_lowercase + "/", min_size=1),
       text(alphabet=ascii_lowercase +"/", min_size=1))
def test_pack_from_cache(data_path, url):
    @patch("cmsis_pack_manager.ZipFile")
    def inner_test(_zf):
        c = cmsis_pack_manager.Cache(True, True, data_path=data_path)
        device = {'pack_file': url}
        c.pack_from_cache(device)
        _zf.called_with(join(data_path, url))
    inner_test()

VERSION_TEMPLATE = (
    "<package>"
    "<vendor>{{vendor}}</vendor>"
    "<name>{{name}}</name>"
    "<url>{{url}}</url>"
    "<releases>"
    "{% for version in versions %}"
    "<release version=\"{{version}}\"></release>"
    "{% endfor %}"
    "</releases>"
    "</package>")

@given(text(alphabet=ascii_lowercase), text(alphabet=ascii_lowercase),
       text(alphabet=ascii_lowercase + ":/_."),
       lists(text(min_size=1), min_size=1))
def test_get_pack_url(name, vendor, url, versions):
    xml = Template(VERSION_TEMPLATE).render(name=name, vendor=vendor, url=url,
                                            versions=versions)
    @patch("cmsis_pack_manager.largest_version")
    def inner_test(largest_version):
        pdsc_contents = BeautifulSoup(xml, "html.parser")
        largest_version.return_value = versions[0]
        c = cmsis_pack_manager.Cache(True, True)
        new_url = c.get_pack_url(pdsc_contents)
        assert new_url.startswith(url)
        assert new_url.endswith("%s.%s.%s.pack" % (vendor, name, versions[0]))
    inner_test()

@given(text(alphabet=ascii_lowercase), text(alphabet=ascii_lowercase),
       text(alphabet=ascii_lowercase + ":/_."),
       text(alphabet=ascii_lowercase + "."))
def test_get_pdsc_url(name, vendor, url, pdsc_filename):
    xml = Template(VERSION_TEMPLATE).render(name=name, vendor=vendor, url=url,
                                            versions=[])
    pdsc_contents = BeautifulSoup(xml, "html.parser")
    c = cmsis_pack_manager.Cache(True, True)
    new_url = c.get_pdsc_url(pdsc_contents, pdsc_filename)
    assert new_url.startswith(url)
    assert new_url.endswith(pdsc_filename)

@given(text(alphabet=ascii_lowercase + ":/_."))
def test_pdsc_to_pack(url):
    @patch("cmsis_pack_manager.Cache.get_pack_url")
    @patch("cmsis_pack_manager.Cache.pdsc_from_cache")
    def inner_test(pdsc_from_cache, get_pack_url):
        c = cmsis_pack_manager.Cache(True, True)
        new_url = c.pdsc_to_pack(url)
        pdsc_from_cache.assert_called_with(url)
        get_pack_url.assert_called_with(pdsc_from_cache.return_value)
    inner_test()

@given(text(alphabet=ascii_lowercase + ":/_."),
       text(alphabet=ascii_lowercase + ":/_."))
def test_cache_pdsc_and_pack(pack_url, pdsc_url):
    @patch("cmsis_pack_manager.Cache.cache_file")
    @patch("cmsis_pack_manager.Cache.pdsc_to_pack")
    def inner_test(pdsc_to_pack, cache_file):
        pdsc_to_pack.return_value = pack_url
        c = cmsis_pack_manager.Cache(True, True)
        c.cache_pdsc_and_pack(pdsc_url)
        cache_file.assert_has_calls([call(pdsc_url), call(pack_url)])
    inner_test()

IDX_TEMPLATE = (
    "{% for name, url in pdscs %}"
    "<pdsc name=\"{{name}}\" url=\"{{url}}\"/>"
    "{% endfor %}")

@given(lists(tuples(text(alphabet=ascii_lowercase, min_size=1),
                    text(alphabet=ascii_lowercase + ":/_.", min_size=1)),
             min_size=1))
def test_get_urls(pdscs):
    xml = Template(IDX_TEMPLATE).render(pdscs=pdscs)
    @patch("cmsis_pack_manager.Cache.pdsc_from_cache")
    def inner_test(pdsc_from_cache):
        pdsc_from_cache.return_value = BeautifulSoup(xml, "html.parser")
        c = cmsis_pack_manager.Cache(True, True)
        urls = c.get_urls()
        for url in urls:
            assert any((url.startswith(pdsc[1].rstrip("/")) and
                        url.endswith(pdsc[0].strip("/"))) for pdsc in pdscs)
    inner_test()
