import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import 'gb_emulator_view.dart';

class GameSelectPage extends StatelessWidget {
  const GameSelectPage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text("Select game"),
      ),
      body: Center(
        child: TextButton(
          child: const Text("Select Game"),
          onPressed: () => _selectGame(context),
        ),
      ),
    );
  }

  void _selectGame(BuildContext context) async {
    final navigator = Navigator.of(context);
    final result = await FilePicker.platform.pickFiles();
    if (result != null) {
      final gamePath = result.files.single.path;
      if (gamePath == null) {
        return;
      }
      if (gamePath.endsWith("gb") || gamePath.endsWith('gbc')) {
        navigator.push(
          MaterialPageRoute(
            builder: (context) => GbEmulatorPage(gamePath: gamePath),
          ),
        );
      }
    }
  }
}

class GbEmulatorPage extends StatefulWidget {
  final String gamePath;

  const GbEmulatorPage({
    Key? key,
    required this.gamePath,
  }) : super(key: key);

  @override
  State<GbEmulatorPage> createState() => _GbEmulatorPageState();
}

class _GbEmulatorPageState extends State<GbEmulatorPage> {
  @override
  Widget build(BuildContext context) {
    const bgColor = Color(0xFFC0C1BD);
    return Scaffold(
      appBar: AppBar(
        title: const Text("Game page"),
        elevation: 0,
        backgroundColor: bgColor,
      ),
      backgroundColor: bgColor,
      body: GbEmulatorView(gamePath: widget.gamePath),
    );
  }
}
