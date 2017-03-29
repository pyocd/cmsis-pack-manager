"""Unit tests for the ArmPackManager module"""

from os.path import join
from string import ascii_lowercase
from mock import patch, MagicMock, call
from hypothesis import given, settings, example
from hypothesis.strategies import booleans, text, lists, just, integers, tuples
from jinja2 import Template
from bs4 import BeautifulSoup

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
@example("http", "google.com://foo")
def test_strip_protocol(protocol, url):
    uri = protocol + "://" + url
    assert(ArmPackManager.strip_protocol(uri) == url)

@given(text(alphabet=ascii_lowercase + ":/_."), text())
def test_cache_file(url, contents):
    @patch("ArmPackManager.Cache.display_counter")
    @patch("ArmPackManager.urlopen")
    @patch("ArmPackManager.makedirs")
    @patch("ArmPackManager.open", create=True)
    def inner_test(_open, _, _urlopen, __):
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
def test_pdsc_to_pack(name, vendor, url, versions):
    xml = Template(VERSION_TEMPLATE).render(name=name, vendor=vendor, url=url,
                                            versions=versions)
    @patch("ArmPackManager.largest_version")
    @patch("ArmPackManager.Cache.pdsc_from_cache")
    def inner_test(pdsc_from_cache, largest_version):
        pdsc_from_cache.return_value = BeautifulSoup(xml, "html.parser")
        largest_version.return_value = versions[0]
        c = ArmPackManager.Cache(True, True)
        new_url = c.pdsc_to_pack(url + "desc.pdsc")
        assert new_url.startswith(url)
        assert new_url.endswith("%s.%s.%s.pack" % (vendor, name, versions[0]))
    inner_test()

@given(text(alphabet=ascii_lowercase + ":/_."),
       text(alphabet=ascii_lowercase + ":/_."))
def test_cache_pdsc_and_pack(pack_url, pdsc_url):
    @patch("ArmPackManager.Cache.cache_file")
    @patch("ArmPackManager.Cache.pdsc_to_pack")
    def inner_test(pdsc_to_pack, cache_file):
        pdsc_to_pack.return_value = pack_url
        c = ArmPackManager.Cache(True, True)
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
    @patch("ArmPackManager.Cache.pdsc_from_cache")
    def inner_test(pdsc_from_cache):
        pdsc_from_cache.return_value = BeautifulSoup(xml, "html.parser")
        c = ArmPackManager.Cache(True, True)
        urls = c.get_urls()
        for url in urls:
            assert any((url.startswith(pdsc[1].rstrip("/")) and
                        url.endswith(pdsc[0].strip("/"))) for pdsc in pdscs)
    inner_test()
