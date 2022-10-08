import 'dart:math';
import 'package:fgb_emu/fgb_emu.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'images.dart';

/// Gameboy emulator view, which include screen and joypad
class GbEmulatorView extends StatefulWidget {
  final String gamePath;

  const GbEmulatorView({Key? key, required this.gamePath}) : super(key: key);

  @override
  State<GbEmulatorView> createState() => _GameViewState();
}

class _GameViewState extends State<GbEmulatorView> with WidgetsBindingObserver {
  late final GbEmulator _emulator;

  @override
  void initState() {
    super.initState();
    _emulator = GbEmulator(gamePath: widget.gamePath);
    _emulator.run();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      _emulator.pause();
    } else if (state == AppLifecycleState.resumed) {
      _emulator.resume();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        /// Screen
        GbLCD(emulator: _emulator),

        /// Brand
        _buildBrand(),

        /// Joypad
        Expanded(
          child: _JoyPad(emulator: _emulator),
        ),
      ],
    );
  }

  /// Build Nintendo Gameboy brand
  Widget _buildBrand() {
    const textColor = Color(0xFF3E3F6F);
    return Container(
      alignment: Alignment.centerLeft,
      padding: const EdgeInsets.only(left: 16, top: 20),
      child: RichText(
        text: const TextSpan(
          children: [
            TextSpan(
              text: "Nintendo ",
              style: TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w800,
                color: textColor,
              ),
            ),
            TextSpan(
              text: "GAME BOY",
              style: TextStyle(
                fontSize: 25,
                fontWeight: FontWeight.w500,
                fontStyle: FontStyle.italic,
                color: textColor,
              ),
            ),
            TextSpan(
              text: "TM",
              style: TextStyle(
                fontSize: 10,
                fontWeight: FontWeight.w400,
                color: textColor,
              ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  void dispose() {
    _emulator.exit();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }
}

/// Gameboy joypad
class _JoyPad extends StatelessWidget {
  final GbEmulator emulator;

  const _JoyPad({
    Key? key,
    required this.emulator,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    const double top = 60;
    const horizontolTop = top + 42;
    const double verticalLeft = 44;
    return Stack(
      children: [
        /// Left key
        Positioned(
          left: 0,
          top: horizontolTop,
          child: _buildArrowButton(GbButton.left),
        ),

        /// Up key
        Positioned(
          left: verticalLeft,
          top: top,
          child: _buildArrowButton(GbButton.up),
        ),

        /// Right key
        Positioned(
          left: 84,
          top: horizontolTop,
          child: _buildArrowButton(GbButton.right),
        ),

        /// Down key
        Positioned(
          left: verticalLeft,
          top: top + 79.5,
          child: _buildArrowButton(GbButton.down),
        ),

        /// B button
        Positioned(
          top: top + 50,
          right: 74,
          child: _buildButton(
            button: GbButton.B,
            child: _buildRoundButton("B"),
          ),
        ),

        /// A button
        Positioned(
          top: top + 10,
          right: 8,
          child: _buildButton(
            button: GbButton.A,
            child: _buildRoundButton("A"),
          ),
        ),

        /// Select button
        Positioned(
          top: top + 180,
          left: screenWidth / 2 - 100,
          child: _buildButton(
            button: GbButton.select,
            child: _buildOvalButton("Select"),
          ),
        ),

        /// Start button
        Positioned(
          top: top + 180,
          left: screenWidth / 2 + 10,
          child: _buildButton(
            button: GbButton.start,
            child: _buildOvalButton("Start"),
          ),
        ),
      ],
    );
  }

  Widget _buildButton({
    required GbButton button,
    required Widget child,
  }) {
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onTapDown: (_) => _onPressButton(button),
      onTapUp: (_) => _onReleaseButton(button),
      onTapCancel: () => _onReleaseButton(button),
      child: child,
    );
  }

  Widget _buildArrowButton(GbButton button) {
    final int quarter;
    switch (button) {
      case GbButton.up:
        quarter = 0;
        break;
      case GbButton.left:
        quarter = 3;
        break;
      case GbButton.right:
        quarter = 1;
        break;
      case GbButton.down:
        quarter = 2;
        break;
      default:
        quarter = 0;
        break;
    }

    Widget _tapArea({
      double? top,
      double? left,
      double? right,
      double? bottom,
      double? width,
      double? height,
    }) {
      return Positioned(
        top: top,
        left: left,
        right: right,
        bottom: bottom,
        child: SizedBox(
          width: width,
          height: height,
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            onTapDown: (_) => _onPressButton(button),
            onTapUp: (_) => _onReleaseButton(button),
            onTapCancel: () => _onReleaseButton(button),
          ),
        ),
      );
    }

    return RotatedBox(
      quarterTurns: quarter,
      child: SizedBox(
        width: 72,
        height: 76,
        child: Stack(
          children: [
            Positioned(
              top: 16,
              left: 16,
              child: Image.asset(
                ImagePath.icArrowButton,
                width: 40,
                height: 60,
                fit: BoxFit.fill,
              ),
            ),
            _tapArea(width: 72, height: 39),
            _tapArea(top: 39, left: 16, right: 16, height: 16),
          ],
        ),
      ),
    );
  }

  void _onPressButton(GbButton button) {
    HapticFeedback.lightImpact();
    emulator.pressButton(button);
  }

  void _onReleaseButton(GbButton button) {
    emulator.releaseButton(button);
  }

  Widget _buildRoundButton(String text) {
    const color = Color(0xFF751C48);
    return Container(
      padding: const EdgeInsets.all(8),
      alignment: Alignment.center,
      child: Container(
        width: 60,
        height: 60,
        alignment: Alignment.center,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: color,
          border: Border.all(
            width: 2,
            color: Colors.white,
          ),
        ),
        child: Text(
          text,
          style: const TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.w800,
            color: Colors.white,
          ),
        ),
      ),
    );
  }

  Widget _buildOvalButton(String text) {
    const color = Color(0xFF6D6B74);
    return Transform.rotate(
      angle: -pi / 6,
      child: Container(
        alignment: Alignment.center,
        padding: const EdgeInsets.all(8),
        child: Container(
          width: 90,
          height: 40,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(20),
            border: Border.all(
              width: 2,
              color: Colors.white,
            ),
          ),
          child: Text(
            text,
            style: const TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w700,
              color: Colors.white,
            ),
          ),
        ),
      ),
    );
  }
}
