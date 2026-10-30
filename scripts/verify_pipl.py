"""Windows版プラグインに格納されたPiPLの構造と必須情報を検証する。"""

import ctypes
from ctypes import wintypes
from pathlib import Path
import struct
import sys


def read_pipl(path):
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.LoadLibraryExW.argtypes = [wintypes.LPCWSTR, wintypes.HANDLE, wintypes.DWORD]
    kernel.LoadLibraryExW.restype = wintypes.HMODULE
    kernel.FindResourceW.argtypes = [wintypes.HMODULE, wintypes.LPCWSTR, wintypes.LPCWSTR]
    kernel.FindResourceW.restype = wintypes.HANDLE
    kernel.SizeofResource.argtypes = [wintypes.HMODULE, wintypes.HANDLE]
    kernel.SizeofResource.restype = wintypes.DWORD
    kernel.LoadResource.argtypes = [wintypes.HMODULE, wintypes.HANDLE]
    kernel.LoadResource.restype = wintypes.HANDLE
    kernel.LockResource.argtypes = [wintypes.HANDLE]
    kernel.LockResource.restype = ctypes.c_void_p
    kernel.FreeLibrary.argtypes = [wintypes.HMODULE]
    kernel.FreeLibrary.restype = wintypes.BOOL

    module = kernel.LoadLibraryExW(str(path.resolve()), None, 2)  # LOAD_LIBRARY_AS_DATAFILE
    if not module:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        name = ctypes.cast(ctypes.c_void_p(16000), wintypes.LPCWSTR)
        resource = kernel.FindResourceW(module, name, "PiPL")
        if not resource:
            raise ctypes.WinError(ctypes.get_last_error())
        size = kernel.SizeofResource(module, resource)
        handle = kernel.LoadResource(module, resource)
        data = kernel.LockResource(handle)
        if not size or not handle or not data:
            raise ValueError("PiPLリソースを読み取れません")
        return ctypes.string_at(data, size)
    finally:
        kernel.FreeLibrary(module)


def verify_pipl(data):
    if len(data) < 10 or data[:6] != b"\x01\x00\x00\x00\x00\x00":
        raise ValueError("PiPLヘッダーが不正です")
    count = struct.unpack_from("<I", data, 6)[0]
    offset = 10
    properties = {}
    for _ in range(count):
        if offset + 16 > len(data):
            raise ValueError("PiPLプロパティのヘッダーが途切れています")
        vendor, key, identifier, length = struct.unpack_from("<4s4sII", data, offset)
        offset += 16
        if vendor != b"MIB8" or identifier != 0 or offset + length > len(data):
            raise ValueError("PiPLプロパティの構造が不正です")
        if key in properties:
            raise ValueError("PiPLプロパティが重複しています")
        properties[key] = data[offset : offset + length]
        offset += length
    if offset != len(data):
        raise ValueError("PiPL末尾に余分なデータがあります")

    name = properties.get(b"ANMe", b"")
    if not name or name[1 : 1 + name[0]] != b"HANAYUKI RGBA Contrast":
        raise ValueError("エフェクトの識別名が不正です")
    flags = properties.get(b"2LGe", b"")
    if len(flags) != 4 or not struct.unpack("<I", flags)[0] & 0x80:
        raise ValueError("折りたたみグループの対応フラグがありません")


if __name__ == "__main__":
    if sys.platform != "win32" or len(sys.argv) != 2:
        raise SystemExit("使用方法: python scripts/verify_pipl.py <Windows版プラグイン>")
    try:
        verify_pipl(read_pipl(Path(sys.argv[1])))
    except (OSError, ValueError, struct.error) as error:
        raise SystemExit(f"PiPL検証失敗: {error}") from error
    print("PiPL検証成功")
