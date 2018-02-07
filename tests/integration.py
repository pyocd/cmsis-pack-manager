try:
    import SimpleHTTPServer as http
except ImportError:
    import http.server as http
try:
    import SocketServer as socketserver
except ImportError:
    import socketserver
import threading
import urllib
import tempfile
from os.path import join, dirname

import cmsis_pack_manager

def test_pull_pdscs():
    PORT = 8001
    handler = http.SimpleHTTPRequestHandler
    httpd = socketserver.TCPServer(("", PORT), handler)
    httpd_thread = threading.Thread(target=httpd.serve_forever)
    httpd_thread.setDaemon(True)
    httpd_thread.start()

    c = cmsis_pack_manager.Cache(True, True, vidx_list=join(dirname(__file__), 'test-pack-index', 'vendors.list'))
    c.data_path = tempfile.mkdtemp()
    c.cache_descriptors()
    assert("MyDevice" in c.index)
