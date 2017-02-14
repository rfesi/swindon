import pytest
import pathlib
import subprocess
import tempfile
import os
import string
import socket

import yarl
import aiohttp

from collections import namedtuple
from aiohttp import web, test_utils

ROOT = pathlib.Path('/work')


def pytest_addoption(parser):
    parser.addoption('--swindon-bin', default=[],
                     action='append',
                     help="Path to swindon binary to run")
    parser.addoption('--swindon-config',
                     default='./tests/config.yaml.tpl',
                     help=("Path to swindon config template,"
                           " default is `%(default)s`"))


SWINDON_BIN = []


def pytest_configure(config):
    bins = config.getoption('--swindon-bin')[:]
    SWINDON_BIN[:] = bins or ['target/debug/swindon']
    for _ in range(len(SWINDON_BIN)):
        p = SWINDON_BIN.pop(0)
        p = ROOT / p
        assert p.exists(), p
        SWINDON_BIN.append(str(p))

# Fixtures


@pytest.fixture(params=[
    'GET', 'PATCH', 'POST', 'PUT', 'UPDATED', 'DELETE', 'XXX'])
def request_method(request):
    """Parametrized fixture changing request method
    (GET / POST / PATCH / ...).
    """
    return request.param


@pytest.fixture(params=[aiohttp.HttpVersion11, aiohttp.HttpVersion10],
                ids=['http/1.1', 'http/1.0'])
def http_version(request):
    return request.param


@pytest.fixture(scope='session', params=[True, False],
                ids=['debug-routing', 'no-debug-routing'])
def debug_routing(request):
    return request.param


@pytest.fixture
def http_request(request_method, http_version, debug_routing):

    async def inner(url):
        async with aiohttp.ClientSession(version=http_version) as s:
            async with s.request(request_method, url) as resp:
                data = await resp.read()
                assert resp.version == http_version
                assert_headers(resp.headers, debug_routing)
                return resp, data
    return inner


def assert_headers(headers, debug_routing):
    assert 'Content-Type' in headers
    assert 'Content-Length' in headers
    assert 'Date' in headers
    assert 'Server' in headers
    if debug_routing:
        assert 'X-Swindon-Route' in headers
    else:
        assert 'X-Swindon-Route' not in headers

    assert len(headers.getall('Content-Type')) == 1
    assert len(headers.getall('Content-Length')) == 1
    assert len(headers.getall('Date')) == 1
    assert headers.getall('Server') == ['swindon/func-tests']


SwindonInfo = namedtuple('SwindonInfo', 'proc url')


@pytest.fixture(scope='session', params=SWINDON_BIN, autouse=True)
def swindon(_proc, request, debug_routing):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('127.0.0.1', 0))
        ADDRESS = s.getsockname()
    addr_str = ':'.join(map(str, ADDRESS))

    swindon_bin = request.param
    fd, fname = tempfile.mkstemp()

    conf_template = pathlib.Path(request.config.getoption('--swindon-config'))
    with (ROOT / conf_template).open('rt') as f:
        tpl = string.Template(f.read())

    config = tpl.substitute(listen_address=addr_str,
                            debug_routing=str(debug_routing).lower(),
                            )
    os.write(fd, config.encode('utf-8'))

    proc = _proc(swindon_bin,
                 '--verbose',
                 '--config',
                 fname,
                 stdout=subprocess.PIPE,
                 stderr=subprocess.PIPE)
    while True:
        assert proc.poll() is None, (
            proc.poll(), proc.stdout.read(), proc.stderr.read())
        line = proc.stdout.readline().decode('utf-8').strip()
        if line.endswith(addr_str):
            break

    url = yarl.URL('http://localhost:{}'.format(ADDRESS[1]))
    try:
        yield SwindonInfo(proc, url)
    finally:
        os.close(fd)
        os.remove(fname)


@pytest.fixture
def swindon_client(loop):
    clients = []

    async def go(__param, *args, **kwargs):
        if not isinstance(__param, web.Application):
            __param = __param(loop, *args, **kwargs)
        client = test_utils.TestClient(__param)
        await client.start_server()
        clients.append(client)
        return client

    async def finalize():
        while clients:
            await (clients.pop()).close()

    try:
        yield go
    finally:
        loop.run_until_complete(finalize())


# helpers


@pytest.fixture(scope='session')
def _proc():
    # Process runner
    processes = []

    def run(*cmdline, **kwargs):
        cmdline = list(map(str, cmdline))
        proc = subprocess.Popen(cmdline, **kwargs)
        processes.append(proc)
        return proc

    try:
        yield run
    finally:
        while processes:
            proc = processes.pop(0)
            proc.terminate()
            proc.wait()