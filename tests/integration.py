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

import cmsis_pack_manager

def test_pull_pdscs():
    PORT = 8001
    handler = http.SimpleHTTPRequestHandler
    httpd = socketserver.TCPServer(("", PORT), handler)
    httpd_thread = threading.Thread(target=httpd.serve_forever)
    httpd_thread.setDaemon(True)
    httpd_thread.start()

    c = cmsis_pack_manager.Cache(True, True)
    c.cache_descriptors() #"localhost:%s" % PORT)
