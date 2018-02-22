try:
    import SimpleHTTPServer as http
except ImportError:
    import http.server as http
try:
    import SocketServer as socketserver
except ImportError:
    import socketserver
import sys
import threading
import urllib
import tempfile
from os.path import join, dirname

import cmsis_pack_manager
import cmsis_pack_manager.pack_manager

def test_pull_pdscs():
    socketserver.TCPServer.allow_reuse_address = True
    PORT = 8001
    handler = http.SimpleHTTPRequestHandler
    httpd = socketserver.TCPServer(("", PORT), handler)
    httpd_thread = threading.Thread(target=httpd.serve_forever)
    httpd_thread.setDaemon(True)
    httpd_thread.start()

    c = cmsis_pack_manager.Cache(
        True, True, json_path=tempfile.mkdtemp(), data_path=tempfile.mkdtemp(),
        vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
    c.cache_everything()
    assert("MyDevice" in c.index)
    assert("MyBoard" in c.aliases)
    assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
    assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))
    c.cache_everything()
    httpd.shutdown()
    assert("MyDevice" in c.index)
    assert("MyBoard" in c.aliases)
    assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
    assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))

def test_pull_pdscs_cli():
    socketserver.TCPServer.allow_reuse_address = True
    PORT = 8001
    handler = http.SimpleHTTPRequestHandler
    httpd = socketserver.TCPServer(("", PORT), handler)
    httpd_thread = threading.Thread(target=httpd.serve_forever)
    httpd_thread.setDaemon(True)
    httpd_thread.start()

    json_path = tempfile.mkdtemp()
    data_path = tempfile.mkdtemp()
    sys.argv = ["pack-manager", "cache", "-e", "--data-path", data_path,
                "--json-path", json_path,
                "--vidx-list", join(dirname(__file__), 'test-pack-index', 'vendors.list')]
    cmsis_pack_manager.pack_manager.main()
    c = cmsis_pack_manager.Cache(True, True, json_path=json_path, data_path=data_path)
    assert("MyDevice" in c.index)
    assert("MyBoard" in c.aliases)
    assert("MyDevice" in c.aliases["MyBoard"]["mounted_devices"])
    assert(c.pack_from_cache(c.index["MyDevice"]).open("MyVendor.MyPack.pdsc"))
