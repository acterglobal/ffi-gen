// AUTO GENERATED FILE, DO NOT EDIT.
//
// Generated by "ffi-gen".

import "dart:convert";
import "dart:ffi" as ffi;
import "dart:io" show Platform;
import "dart:typed_data";

class _Slice extends ffi.Struct {
  external ffi.Pointer<ffi.Uint8> ptr;

  @ffi.IntPtr()
  external int len;
}

class _Alloc extends ffi.Struct {
  external ffi.Pointer<ffi.Uint8> ptr;

  @ffi.IntPtr()
  external int len;

  @ffi.IntPtr()
  external int cap;
}

class Box {
  final Api _api;
  final ffi.Pointer<ffi.Void> _ptr;
  final String _drop_symbol;
  bool _dropped;
  bool _moved;

  Box(this._api, this._ptr, this._drop_symbol)
      : _dropped = false,
        _moved = false;

  late final _dropPtr = this
      ._api
      ._lookup<ffi.NativeFunction<ffi.Void Function(ffi.Pointer<ffi.Void>)>>(
          this._drop_symbol);

  late final _drop =
      _dropPtr.asFunction<void Function(ffi.Pointer<ffi.Void>)>();

  ffi.Pointer<ffi.Void> borrow() {
    if (this._dropped) {
      throw new StateError("use after free");
    }
    if (this._moved) {
      throw new StateError("use after move");
    }
    return this._ptr;
  }

  ffi.Pointer<ffi.Void> move() {
    if (this._dropped) {
      throw new StateError("use after free");
    }
    if (this._moved) {
      throw new StateError("can't move value twice");
    }
    this._moved = true;
    return this._ptr;
  }

  void drop() {
    if (this._dropped) {
      throw new StateError("double free");
    }
    if (this._moved) {
      throw new StateError("can't drop moved value");
    }
    this._dropped = true;
    this._drop(this._ptr);
  }
}

class Api {
  /// Holds the symbol lookup function.
  final ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
      _lookup;

  /// The symbols are looked up in [dynamicLibrary].
  Api(ffi.DynamicLibrary dynamicLibrary) : _lookup = dynamicLibrary.lookup;

  /// The symbols are looked up with [lookup].
  Api.fromLookup(
      ffi.Pointer<T> Function<T extends ffi.NativeType>(String symbolName)
          lookup)
      : _lookup = lookup;

  /// The library is loaded from the executable.
  factory Api.loadStatic() {
    return Api(ffi.DynamicLibrary.executable());
  }

  /// The library is dynamically loaded.
  factory Api.loadDynamic(String name) {
    return Api(ffi.DynamicLibrary.open(name));
  }

  /// The library is loaded based on platform conventions.
  factory Api.load() {
    String? name;
    if (Platform.isLinux) name = "libapi.so";
    if (Platform.isAndroid) name = "libapi.so";
    if (Platform.isMacOS) name = "libapi.dylib";
    if (Platform.isIOS) name = "\"\"";
    if (Platform.isWindows) "api.dll";
    if (name == null) {
      throw UnsupportedError("\"This platform is not supported.\"");
    }
    if (name == "") {
      return Api.loadStatic();
    } else {
      return Api.loadDynamic(name);
    }
  }

  late final _allocatePtr = _lookup<
      ffi.NativeFunction<
          ffi.Pointer<ffi.Uint8> Function(ffi.IntPtr, ffi.IntPtr)>>("allocate");

  late final _allocate =
      _allocatePtr.asFunction<ffi.Pointer<ffi.Uint8> Function(int, int)>();

  ffi.Pointer<T> allocate<T extends ffi.NativeType>(
      int byteCount, int alignment) {
    return _allocate(byteCount, alignment).cast();
  }

  late final _deallocatePtr = _lookup<
      ffi.NativeFunction<
          ffi.Void Function(
              ffi.Pointer<ffi.Uint8>, ffi.IntPtr, ffi.IntPtr)>>("deallocate");

  late final _deallocate =
      _deallocatePtr.asFunction<Function(ffi.Pointer<ffi.Uint8>, int, int)>();

  void deallocate<T extends ffi.NativeType>(
      ffi.Pointer pointer, int byteCount, int alignment) {
    this._deallocate(pointer.cast(), byteCount, alignment);
  }

  void hello_world() {
    final ret = _hello_world();
  }

  late final _hello_worldPtr =
      _lookup<ffi.NativeFunction<ffi.Void Function()>>("__hello_world");

  late final _hello_world = _hello_worldPtr.asFunction<void Function()>();
}
