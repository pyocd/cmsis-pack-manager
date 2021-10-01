try:
    import SimpleHTTPServer as http
except ImportError:
    import http.server as http
try:
    import SocketServer as socketserver
except ImportError:
    import socketserver
import contextlib
import os
import sys
import threading
import tempfile
from os.path import join, dirname, exists

import cmsis_pack_manager
import cmsis_pack_manager.pack_manager


MODULE_ROOT = join(dirname(__file__), "..")


@contextlib.contextmanager
def pushd(new_dir):
    previous_dir = os.getcwd()
    os.chdir(new_dir)
    try:
        yield
    finally:
        os.chdir(previous_dir)


@contextlib.contextmanager
def cmsis_server():
    with pushd(MODULE_ROOT):
        print(os.getcwd())
        socketserver.TCPServer.allow_reuse_address = True
        PORT = 8001
        handler = http.SimpleHTTPRequestHandler
        httpd = socketserver.TCPServer(("", PORT), handler)
        httpd_thread = threading.Thread(target=httpd.serve_forever)
        httpd_thread.setDaemon(True)
        httpd_thread.start()
        try:
            yield
        finally:
            httpd.shutdown()


def test_empyt_cache():
    json_path = tempfile.mkdtemp()
    data_path = tempfile.mkdtemp()
    c = cmsis_pack_manager.Cache(
        True, True, json_path=json_path, data_path=data_path,
        vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
    try:
        c.index
    except Exception:
        assert False, "Unexpected exception raised on an empty cache"


def test_pull_pdscs():
    with cmsis_server():
        json_path = tempfile.mkdtemp()
        data_path = tempfile.mkdtemp()
        c = cmsis_pack_manager.Cache(
            True, True, json_path=json_path, data_path=data_path,
            vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
        c.cache_everything()
        assert("MyDevice" in c.index)
        assert("MyFamily" == c.index["MyDevice"]["family"])
        assert("MyBoard" in c.aliases)
        assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
        assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))
        c = cmsis_pack_manager.Cache(
            True, True, json_path=json_path, data_path=data_path,
            vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
        c.cache_everything()
        assert("MyDevice" in c.index)
        assert("MyFamily" == c.index["MyDevice"]["family"])
        assert("MyBoard" in c.aliases)
        assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
        assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))

def test_install_pack():
    with cmsis_server():
        json_path = tempfile.mkdtemp()
        data_path = tempfile.mkdtemp()
        c = cmsis_pack_manager.Cache(
            True, True, json_path=json_path, data_path=data_path,
            vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
        c.cache_descriptors()
        packs = c.packs_for_devices([c.index["MyDevice"]])
        c.download_pack_list(packs)
        assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))

def test_pull_pdscs_cli():
    with cmsis_server():
        json_path = tempfile.mkdtemp()
        data_path = tempfile.mkdtemp()
        sys.argv = ["pack-manager", "cache", "everything", "--data-path", data_path,
                    "--json-path", json_path,
                    "--vidx-list", join(dirname(__file__), 'test-pack-index', 'vendors.list')]
        cmsis_pack_manager.pack_manager.main()
        c = cmsis_pack_manager.Cache(True, True, json_path=json_path, data_path=data_path)
        assert("MyDevice" in c.index)
        assert("MyBoard" in c.aliases)
        assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
        assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))

def test_add_pack_from_path():
    json_path = tempfile.mkdtemp()
    data_path = tempfile.mkdtemp()
    c = cmsis_pack_manager.Cache(
        True, True, json_path=json_path, data_path=data_path)
    c.add_pack_from_path(join(dirname(__file__), 'test-pack-index', 'MyVendor.MyPack.pdsc'))
    assert("MyDevice" in c.index)
    assert("MyBoard" in c.aliases)
    assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])

def test_add_pack_from_path_cli():
    json_path = tempfile.mkdtemp()
    data_path = tempfile.mkdtemp()
    sys.argv = ["pack-manager", "add-packs",
                join(dirname(__file__), 'test-pack-index', 'MyVendor.MyPack.pdsc'),
                "--data-path", data_path,
                "--json-path", json_path,
                "--vidx-list", join(dirname(__file__), 'test-pack-index', 'vendors.list')]
    cmsis_pack_manager.pack_manager.main()
    c = cmsis_pack_manager.Cache(True, True, json_path=json_path, data_path=data_path)
    assert("MyDevice" in c.index)
    assert("MyBoard" in c.aliases)
    assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])

def test_dump_parts_cli():
    with cmsis_server():
        json_path = tempfile.mkdtemp()
        data_path = tempfile.mkdtemp()
        sys.argv = ["pack-manager", "cache", "packs", "--data-path", data_path,
                    "--json-path", json_path,
                    "--vidx-list", join(dirname(__file__), 'test-pack-index', 'vendors.list')]
        cmsis_pack_manager.pack_manager.main()
        dump_path = tempfile.mkdtemp()
        sys.argv = ["pack-manager", "dump-parts", dump_path, "Dev",
                    "--data-path", data_path,
                    "--json-path", json_path]
        cmsis_pack_manager.pack_manager.main()
        c = cmsis_pack_manager.Cache(True, True, json_path=json_path, data_path=data_path)
        assert exists(join(dump_path, "index.json"))
        for algo in c.index["MyDevice"]["algorithms"]:
            assert exists(join(dump_path, algo["file_name"]))


def test_panic_handling():
    from cmsis_pack_manager import ffi
    c = cmsis_pack_manager.Cache(
        True, True, json_path=tempfile.mkdtemp(), data_path=tempfile.mkdtemp(),
        vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
    try:
        c._call_rust_parse(ffi.NULL)
        assert False
    except:
        pass

def test_print_cache_dir_cli(capsys):
    sys.argv = ["pack-manager", "print-cache-dir"]
    cmsis_pack_manager.pack_manager.main()
    captured = capsys.readouterr()

    c = cmsis_pack_manager.Cache(True, True)

    assert (c.data_path ==  captured.out.strip())
