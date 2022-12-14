import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;
import 'dart:ffi' as ffi;

import 'package:ffi/ffi.dart';
import 'package:fgb_emu/src/ffi/ffi_binding.dart';
import 'package:fgb_emu/src/utils/logger.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../ffi/native_binding.dart';

typedef _NativeEmulator = ffi.Pointer<Emulator_C>;

const int gbWindowWidth = 160;
const int gbWindowHeight = 144;

enum GbButton {
  left(GbBtn.LEFT),
  up(GbBtn.UP),
  right(GbBtn.RIGHT),
  down(GbBtn.DOWN),
  A(GbBtn.A),
  B(GbBtn.B),
  select(GbBtn.SELECT),
  start(GbBtn.START);

  final int val;

  const GbButton(this.val);
}

class GbEmulator {
  final String gamePath;

  /// Native implemented emulator
  late final _NativeEmulator _emulator;

  /// The width of emulator's screen
  late final double _windowWidth;

  late final GbWindowBufferNotifier _notifier;

  late final ffi.Pointer<WindowConfig> _windowConfig;

  GbWindowBufferNotifier get windowBufferNotifier => _notifier;

  double get windowWidth => _windowWidth;

  /// The height of emulator's screen
  late final double _windowHeight;

  double get windowHeight => _windowHeight;

  /// The emulator screen scale factor according to original size
  late final int _windowScaleFactor;

  /// Emulator screen pixels data, will get update each frame
  ui.Image? _buffer;

  ui.Image? get buffer => _buffer;

  SharedPreferences? _pref;

  Future<SharedPreferences> get pref async {
    _pref ??= await SharedPreferences.getInstance();
    return _pref!;
  }

  GbEmulator({
    required this.gamePath,
  }) {
    // Init logger
    FLogger.init();
    _notifier = GbWindowBufferNotifier();
    // Calculate window scale factor, width and height
    final screenWidth =
        MediaQueryData.fromWindow(WidgetsBinding.instance.window).size.width;
    _windowScaleFactor = (screenWidth / gbWindowWidth).floor();
    _windowWidth = gbWindowWidth.toDouble() * _windowScaleFactor;
    _windowHeight = gbWindowHeight.toDouble() * _windowScaleFactor;
    _windowConfig = malloc<WindowConfig>();
    _windowConfig.ref.scale_factor = _windowScaleFactor.toDouble();
    // Create emulator
    _emulator = FFIBinding.binding.create_emulator(_windowConfig);
  }

  void run() async {
    final saveDir = await findSaveDir();
    FFIBinding.binding.run_emulator(
      _emulator,
      gamePath.toNativeUtf8().cast(),
      saveDir.toNativeUtf8().cast(),
    );
  }

  void pause() {
    FFIBinding.binding.pause_emulator(_emulator);
  }

  void resume() {
    FFIBinding.binding.resume_emulator(_emulator);
  }

  void exit() {
    FFIBinding.binding.exit_emulator(_emulator);
    _buffer?.dispose();
    malloc.free(_windowConfig);
  }

  void pressButton(GbButton button) {
    FFIBinding.binding.press_button(_emulator, button.val);
  }

  void releaseButton(GbButton button) {
    FFIBinding.binding.release_button(_emulator, button.val);
  }

  void updateWindowBuffer() async {
    final c = Completer<ui.Image>();
    final bufferSize = (_windowWidth * _windowHeight).toInt();
    final pixels =
        FFIBinding.binding.get_window_buffer(_emulator).asTypedList(bufferSize);
    void decodeCallback(ui.Image image) {
      c.complete(image);
    }

    ui.decodeImageFromPixels(
      pixels.buffer.asUint8List(),
      _windowWidth.toInt(),
      _windowHeight.toInt(),
      ui.PixelFormat.rgba8888,
      decodeCallback,
    );
    final newBuffer = await c.future;
    _buffer = newBuffer;
    _notifier.notifyBufferUpdate();
  }

  /// Get the directory where to save game data (e.g. ram, rtc)
  Future<String> findSaveDir() async {
    final saveDir = (await pref).getString(gamePath);
    if (saveDir != null) {
      // Game have been saved before
      return saveDir;
    }
    // Game have not been saved, create a new save directory
    final start = gamePath.lastIndexOf('/') + 1;
    final end = gamePath.lastIndexOf('.');
    assert(start != -1 && end != -1 && end > start);
    // Game name without extension (like .gb and .gbc)
    final gameName = gamePath.substring(start, end);
    final appDocDir = await getApplicationDocumentsDirectory();
    var newSaveDir = "${appDocDir.path}/game/$gameName";
    var suffix = 1;
    // Solve file conflict (different rom path refer to same game)
    while (await File(newSaveDir).exists()) {
      // Increase suffix until file is not exist
      newSaveDir = "${newSaveDir}_$suffix";
      suffix++;
    }
    return newSaveDir;
  }
}

/// Used to notifiy window buffer change
class GbWindowBufferNotifier extends ChangeNotifier {
  void notifyBufferUpdate() {
    notifyListeners();
  }
}
