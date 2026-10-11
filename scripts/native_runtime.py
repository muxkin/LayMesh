"""Inspect Linux engine dependencies without executing a foreign binary."""
import struct


def check_linux_runtime(data):
    assert data[:6] == b'\x7fELF\x02\x01', 'Expected little-endian ELF64'
    offset = struct.unpack_from('<Q', data, 40)[0]
    size, count = struct.unpack_from('<HH', data, 58)
    assert size >= 64 and count > 0 and offset + size * count <= len(data)
    sections = [struct.unpack_from('<IIQQQQIIQQ', data, offset + i * size) for i in range(count)]
    dynamic = [s for s in sections if s[1] == 6]
    assert len(dynamic) == 1, 'Missing or ambiguous ELF dynamic section'
    section = dynamic[0]
    strings = sections[section[6]]
    table = data[strings[4]:strings[4] + strings[5]]
    names = []
    for at in range(section[4], section[4] + section[5], section[9]):
        tag, value = struct.unpack_from('<qQ', data, at)
        if tag == 1:
            names.append(table[value:table.index(b'\0', value)].decode('ascii'))
    # All font/codec helpers are static. Only the baseline GNU/C++ runtime
    # libraries permitted by manylinux may remain in a standalone engine.
    allowed = {'libc.so.6', 'libm.so.6', 'libstdc++.so.6', 'libgcc_s.so.1',
               'libpthread.so.0', 'libdl.so.2', 'librt.so.1',
               'ld-linux-x86-64.so.2', 'ld-linux-aarch64.so.1'}
    assert names and set(names) <= allowed, f'Unbundled Linux runtime dependencies: {sorted(set(names) - allowed)}'
    return names
