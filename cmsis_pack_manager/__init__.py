# ARM Pack Manager
# Copyright (c) 2017 ARM Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from xdg.BaseDirectory import save_data_path
from urllib2 import urlopen, URLError
from bs4 import BeautifulSoup
from os.path import join, dirname, basename, exists
from os import makedirs
from errno import EEXIST
from threading import Thread
from Queue import Queue
from re import compile, sub
from sys import stderr, stdout
from itertools import takewhile
import argparse
from json import dump, load
from zipfile import ZipFile
import warnings
from distutils.version import LooseVersion
from shutil import copyfile

warnings.filterwarnings("ignore")

from fuzzywuzzy import process

RootPackURL = "http://www.keil.com/pack/index.idx"


protocol_matcher = compile("\w*://")
def strip_protocol(url) :
    return protocol_matcher.sub("", str(url), count=1)

def largest_version(versions) :
    return sorted(versions, reverse=True, key=lambda v: LooseVersion(v))[0]

def do_queue(Class, function, interable) :
    q = Queue()
    threads = [Class(q, function) for each in range(20)]
    for each in threads :
        each.setDaemon(True)
        each.start()
    for thing in interable :
        q.put(thing)
    q.join()

class Reader (Thread) :
    def __init__(self, queue, func) :
        Thread.__init__(self)
        self.queue = queue
        self.func = func
    def run(self) :
        while True :
            url = self.queue.get()
            self.func(url)
            self.queue.task_done()


class Cache () :
    """ The Cache object is the only relevant API object at the moment

    Constructing the Cache object does not imply any caching.
    A user of the API must explicitly call caching functions.

    :param silent: A boolean that, when True, significantly reduces the printing of this Object
    :type silent: bool
    :param no_timeouts: A boolean that, when True, disables the default connection timeout and low speed timeout for downloading things.
    :type no_timeouts: bool
    """
    def __init__ (self, silent, no_timeouts, json_path=None, data_path=None) :
        default_path = save_data_path('arm-pack-manager')
        json_path = default_path if not json_path else json_path
        self.silent = silent
        self.counter = 0
        self.total = 1
        self._index = {}
        self._aliases = {}
        self.urls = None
        self.no_timeouts = no_timeouts
        self.index_path = join(json_path, "index.json")
        self.aliases_path = join(json_path, "aliases.json")
        self.data_path = default_path if not data_path else data_path

    def display_counter (self, message) :
        stdout.write("{} {}/{}\r".format(message, self.counter, self.total))
        stdout.flush()

    def _cache_lookup(self, url):
        """Low level lookup of a file in the cache

        Note: Does not check if the file exists

        :param url: The URL to lookup in the cache.
        :type url: str
        :rtype: FileString
        """
        return join(self.data_path, strip_protocol(url))

    def cache_file (self, url) :
        """Low level interface to caching a single file.

        :param url: The URL to cache.
        :type url: str
        :rtype: None
        """
        if not self.silent : print("Caching {}...".format(url))
        dest = self._cache_lookup(url)
        try :
            makedirs(dirname(dest))
        except OSError as exc :
            if exc.errno == EEXIST : pass
            else : raise
        try:
            with open(dest, "wb+") as fd :
                fd.write(urlopen(url).read())
        except URLError as e:
            stderr.write(e.reason)
        self.counter += 1
        self.display_counter("Caching Files")

    def pdsc_to_pack (self, url) :
        """Find the URL of the specified pack file described by a PDSC URL.

        The PDSC is assumed to be cached and is looked up in the cache by its URL.

        :param url: The url used to look up the PDSC.
        :type url: str
        :return: The url of the PACK file.
        :rtype: str
        """
        content = self.pdsc_from_cache(url)
        return self.get_pack_url(content)

    @staticmethod
    def get_pdsc_url(content, filename):
        """Find the URL of the specified PDSC file described by a PDSC.

        :param content: The contents of a PDSC file
        :type content: BeautifulSoup
        :return: The url of the PDSC file.
        :rtype: str
        """
        new_url = content.package.url.get_text()
        if not new_url.endswith("/") :
            new_url = new_url + "/"
        return new_url + filename

    @staticmethod
    def get_pack_url(content):
        """Find the URL of the specified pack file described by a PDSC.

        :param content: The contents of a PDSC file
        :type content: BeautifulSoup
        :return: The url of the PACK file.
        :rtype: str
        """
        new_url = content.package.url.get_text()
        if not new_url.endswith("/") :
            new_url = new_url + "/"
        return (new_url + content.package.vendor.get_text() + "." +
                content.package.find('name').get_text() + "." +
                largest_version([t['version'] for t in
                                 content.package.releases('release')]) + ".pack")

    def cache_pdsc_and_pack (self, url) :
        self.cache_file(url)
        try :
            self.cache_file(self.pdsc_to_pack(url))
        except AttributeError :
            stderr.write("[ ERROR ] {} does not appear to be a conforming .pdsc file\n".format(url))
            self.counter += 1

    def get_urls(self):
        """Extract the URLs of all know PDSC files.

        Will pull the index from the internet if it is not cached.

        :return: A list of all PDSC URLs
        :rtype: [str]
        """
        if not self.urls :
            try : root_data = self.pdsc_from_cache(RootPackURL)
            except IOError : root_data = self.cache_and_parse(RootPackURL)
            self.urls = ["/".join([pdsc.get('url').rstrip("/"),
                                   pdsc.get('name').strip("/")])
                         for pdsc in root_data.find_all("pdsc")]
        return self.urls

    def _extract_dict(self, device, filename, pack) :
        to_ret = dict(pdsc_file=filename, pack_file=pack)
        try : to_ret["memory"] = dict([(m["id"], dict(start=m["start"],
                                                      size=m["size"]))
                                       for m in device("memory")])
        except (KeyError, TypeError, IndexError) as e : pass
        try: algorithms = device("algorithm")
        except:
            try: algorithms = device.parent("algorithm")
            except: pass
        else:
            if not algorithms:
                try: algorithms = device.parent("algorithm")
                except: pass
        try : to_ret["algorithm"] = dict([(algo.get("name").replace('\\','/'),
                                           dict(start=algo["start"],
                                                size=algo["size"],
                                                ramstart=algo.get("ramstart",None),
                                                ramsize=algo.get("ramsize",None),
                                                default=algo.get("default",1)))
                                       for algo in algorithms])
        except (KeyError, TypeError, IndexError) as e: pass
        try: to_ret["debug"] = device.parent.parent.debug["svd"]
        except (KeyError, TypeError, IndexError) as e : pass
        try: to_ret["debug"] = device.parent.debug["svd"]
        except (KeyError, TypeError, IndexError) as e : pass
        try: to_ret["debug"] = device.debug["svd"]
        except (KeyError, TypeError, IndexError) as e : pass

        to_ret["compile"] = {}
        try: compile_l1 = device.parent("compile")
        except (KeyError, TypeError, IndexError) as e : compile_l1 = []
        try: compile_l2 = device.parent.parent("compile")
        except (KeyError, TypeError, IndexError) as e : compile_l2 = []
        compile = compile_l2 + compile_l1
        for c in compile:
            try: to_ret["compile"]["header"] = c["header"]
            except (KeyError, TypeError, IndexError) as e : pass
            try: to_ret["compile"]["define"] =  c["define"]
            except (KeyError, TypeError, IndexError) as e : pass

        try: to_ret["core"] = device.parent.processor['dcore']
        except (KeyError, TypeError, IndexError) as e : pass
        try: to_ret["core"] = device.parent.parent.processor['dcore']
        except (KeyError, TypeError, IndexError) as e : pass

        to_ret["processor"] = {}
        try: proc_l1 = device("processor")
        except (KeyError, TypeError, IndexError) as e: proc_l1 = []
        try: proc_l2 = device.parent("processor")
        except (KeyError, TypeError, IndexError) as e: proc_l2 = []
        try: proc_l3 = device.parent.parent("processor")
        except (KeyError, TypeError, IndexError) as e: proc_l3 = []
        proc = proc_l3 + proc_l2 + proc_l1
        for p in proc:
            try: to_ret["processor"]["fpu"] = p['dfpu']
            except (KeyError, TypeError, IndexError) as e: pass
            try: to_ret["processor"]["endianness"] = p['dendian']
            except (KeyError, TypeError, IndexError) as e: pass
            try: to_ret["processor"]["clock"] = p['dclock']
            except (KeyError, TypeError, IndexError) as e: pass

        try: to_ret["vendor"] = device.parent['dvendor']
        except (KeyError, TypeError, IndexError) as e: pass
        try: to_ret["vendor"] = device.parent.parent['dvendor']
        except (KeyError, TypeError, IndexError) as e: pass

        if not to_ret["processor"]:
            del to_ret["processor"]

        if not to_ret["compile"]:
            del to_ret["compile"]

        to_ret['debug-interface'] = []

        return to_ret

    def _merge_targets(self, pdsc_url, pack_url, pdsc_contents):
        to_merge = {}
        for device in pdsc_contents("device"):
            to_merge[device['dname']] = self._extract_dict(
                device, pdsc_url, pack_url)
        self._index.update(to_merge)

    def _generate_index_helper(self, pdsc_url) :
        try :
            pack_url = self.pdsc_to_pack(pdsc_url)
            pdsc_contens = self.pdsc_from_cache(pdsc_url)
            self._merge_targets(pdsc_url, pack_url, pdsc_contents)
        except AttributeError as e :
            stderr.write("[ ERROR ] file {}\n".format(pdsc_url))
            print(e)
        self.counter += 1
        self.display_counter("Generating Index")

    def _generate_aliases_helper(self, d) :
        try :
            mydict = []
            for dev in self.pdsc_from_cache(d)("board"):
                try :
                    mydict.append((dev['name'], dev.mounteddevice['dname']))
                except (KeyError, TypeError, IndexError) as e:
                    pass
            self._aliases.update(dict(mydict))
        except (AttributeError, TypeError) as e :
            pass
        self.counter += 1
        self.display_counter("Scanning for Aliases")

    def get_flash_algorthim_binary(self, device_name, all=False) :
        """Retrieve the flash algorithm file for a particular part.

        Assumes that both the PDSC and the PACK file associated with that part are in the cache.

        :param device_name: The exact name of a device
        :param all: Return an iterator of all flash algos for this device
        :type device_name: str
        :return: A file-like object that, when read, is the ELF file that describes the flashing algorithm
        :return: A file-like object that, when read, is the ELF file that describes the flashing algorithm.
                 When "all" is set to True then an iterator for file-like objects is returned
        :rtype: ZipExtFile or ZipExtFile iterator if all is True
        """
        device = self.index[device_name]
        pack = self.pack_from_cache(device)
        algo_itr = (pack.open(path) for path in device['algorithm'].keys())
        return algo_itr if all else algo_itr.next()

    def get_svd_file(self, device_name) :
        """Retrieve the flash algorithm file for a particular part.

        Assumes that both the PDSC and the PACK file associated with that part are in the cache.

        :param device_name: The exact name of a device
        :type device_name: str
        :return: A file-like object that, when read, is the ELF file that describes the flashing algorithm
        :rtype: ZipExtFile
        """
        device = self.index[device_name]
        pack = self.pack_from_cache(device)
        return pack.open(device['debug'])

    def generate_index(self) :
        self._index = {}
        self.counter = 0
        do_queue(Reader, self._generate_index_helper, self.get_urls())
        with open(self.index_path, "wb+") as out:
            self._index["version"] = "0.1.0"
            dump(self._index, out)
        stdout.write("\n")

    def generate_aliases(self) :
        self._aliases = {}
        self.counter = 0
        do_queue(Reader, self._generate_aliases_helper, self.get_urls())
        with open(self.aliases_path, "wb+") as out:
            dump(self._aliases, out)
        stdout.write("\n")

    def find_device(self, match) :
        choices = process.extract(match, self.index.keys(), limit=len(self.index))
        choices = sorted([(v, k) for k, v in choices], reverse=True)
        if choices : choices = list(takewhile(lambda t: t[0] == choices[0][0], choices))
        return [(v, self.index[v]) for k,v in choices]

    def dump_index_to_file(self, file) :
        with open(file, "wb+") as out:
            dump(self.index, out)

    @property
    def index(self) :
        """An index of most of the important data in all cached PDSC files.

        :Example:

        >>> from ArmPackManager import Cache
        >>> a = Cache()
        >>> a.index["LPC1768"]
        {u'algorithm': {u'RAMsize': u'0x0FE0',
                u'RAMstart': u'0x10000000',
                u'name': u'Flash/LPC_IAP_512.FLM',
                u'size': u'0x80000',
                u'start': u'0x00000000'},
         u'compile': [u'Device/Include/LPC17xx.h', u'LPC175x_6x'],
         u'debug': u'SVD/LPC176x5x.svd',
         u'pdsc_file': u'http://www.keil.com/pack/Keil.LPC1700_DFP.pdsc',
         u'memory': {u'IRAM1': {u'size': u'0x8000', u'start': u'0x10000000'},
                     u'IRAM2': {u'size': u'0x8000', u'start': u'0x2007C000'},
                     u'IROM1': {u'size': u'0x80000', u'start': u'0x00000000'}}}


        """
        if not self._index :
            with open(self.index_path) as i :
                self._index = load(i)
        return self._index
    @property
    def aliases(self) :
        """An index of most of the important data in all cached PDSC files.

        :Example:

        >>> from ArmPackManager import Cache
        >>> a = Cache()
        >>> a.index["LPC1768"]
        {u'algorithm': {u'RAMsize': u'0x0FE0',
                u'RAMstart': u'0x10000000',
                u'name': u'Flash/LPC_IAP_512.FLM',
                u'size': u'0x80000',
                u'start': u'0x00000000'},
         u'compile': [u'Device/Include/LPC17xx.h', u'LPC175x_6x'],
         u'debug': u'SVD/LPC176x5x.svd',
         u'pdsc_file': u'http://www.keil.com/pack/Keil.LPC1700_DFP.pdsc',
         u'memory': {u'IRAM1': {u'size': u'0x8000', u'start': u'0x10000000'},
                     u'IRAM2': {u'size': u'0x8000', u'start': u'0x2007C000'},
                     u'IROM1': {u'size': u'0x80000', u'start': u'0x00000000'}}}


        """
        if not self._aliases :
            with open(self.aliases_path) as i :
                self._aliases = load(i)
        return self._aliases

    def cache_everything(self) :
        """Cache every PACK and PDSC file known.

        Generates an index afterwards.

        .. note:: This process may use 4GB of drive space and take upwards of 10 minutes to complete.
        """
        self.cache_pack_list(self.get_urls())
        self.generate_index()
        self.generate_aliases()

    def cache_descriptors(self) :
        """Cache every PDSC file known.

        Generates an index afterwards.

        .. note:: This process may use 11MB of drive space and take upwards of 1 minute.
        """
        self.cache_descriptor_list(self.get_urls())
        self.generate_index()
        self.generate_aliases()

    def cache_descriptor_list(self, list) :
        """Cache a list of PDSC files.

        :param list: URLs of PDSC files to cache.
        :type list: [str]
        """
        self.total = len(list)
        self.display_counter("Caching Files")
        do_queue(Reader, self.cache_file, list)
        stdout.write("\n")

    def cache_pack_list(self, list) :
        """Cache a list of PACK files, referenced by their PDSC URL

        :param list: URLs of PDSC files to cache.
        :type list: [str]
        """
        self.total = len(list) * 2
        self.display_counter("Caching Files")
        do_queue(Reader, self.cache_pdsc_and_pack, list)
        stdout.write("\n")

    def pdsc_from_cache(self, url) :
        """Low level inteface for extracting a PDSC file from the cache.

        Assumes that the file specified is a PDSC file and is in the cache.

        :param url: The URL of a PDSC file.
        :type url: str
        :return: A parsed representation of the PDSC file.
        :rtype: BeautifulSoup
        """
        dest = join(self.data_path, strip_protocol(url))
        with open(dest, "r") as fd :
            return BeautifulSoup(fd, "html.parser")

    def pack_from_cache(self, device) :
        """Low level inteface for extracting a PACK file from the cache.

        Assumes that the file specified is a PACK file and is in the cache.

        :param url: The URL of a PACK file.
        :type url: str
        :return: A parsed representation of the PACK file.
        :rtype: ZipFile
        """
        return ZipFile(join(self.data_path,
                            strip_protocol(device['pack_file'])))

    def gen_dict_from_cache() :
        pdsc_files = pdsc_from_cache(RootPackUrl)

    def cache_and_parse(self, url) :
        """A low level shortcut that Caches and Parses a PDSC file.

        :param url: The URL of the PDSC file.
        :type url: str
        :return: A parsed representation of the PDSC file.
        :rtype: BeautifulSoup
        """
        self.cache_file(url)
        return self.pdsc_from_cache(url)

    @staticmethod
    def find_pdsc(zipfile):
        """Find the PDSC file within a PACK file

        :param zipfile: The PACK to scan
        :type zipfile: ZipFile
        :return: The location of the PDSC file within the PACK file
        :rtype: str
        """
        for zipinfo in zipfile.infolist():
            if (zipinfo.filename.upper().endswith(".PDSC")):
                return zipinfo.filename
        return None

    def add_local_pack_file(self, filename):
        """Add a single pack file to the index

        :param filename: The pack file to add to the index
        """
        _ = self.index # Force the cache to be loaded
        zipfile = ZipFile(open(filename), "rb")
        pdsc_filename = self.find_pdsc(zipfile)
        if not pdsc_filename:
            raise Exception("PDSC file not found in PACK %s" % filename)
        with zipfile.open(pdsc_filename) as pdsc:
            pdsc_content = BeautifulSoup(pdsc, "html.parser")
        pdsc_url = self.get_pdsc_url(pdsc_contents, pdsc_filename)
        pack_url = self.get_pack_url(pdsc_contents)
        self._merge_target(pdsc_url, pack_url, pdsc_content)
        pack_loc = self._cache_lookup(pack_url)
        if not exists(dirname(pack_loc)):
            makedirs(dirname(pack_loc))
        copyfile(filename, pack_loc)
