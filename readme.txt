Kiara Watson
COSC 455 Project 1 - LOLCODE Markdown Compiler

Public GitHub repository:
PENDING - add the public repository URL after GitHub sign-in.

Target operating system:
macOS on Apple silicon (ARM64). The executable is named lolcompiler and is stored in bin/.

AI/LLM use:
AI/LLM tools were used to compare the implementation with the written specification, suggest edge-case tests, and assist with test scaffolding and documentation. The final behavior was checked by compiling the program and running both Rust unit tests and command-line input tests. A project-specific interaction transcript is included in design/AI_LLM_Use_Transcript.txt.

Build:
cargo build --release

Run:
./bin/lolcompiler input.lol

The compiler writes input.html beside the source file and opens it in the default browser after a successful compilation.
