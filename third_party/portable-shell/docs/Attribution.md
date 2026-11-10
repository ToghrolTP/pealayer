# Attribution

PortableShell is distributed under the [MIT license](../LICENSE); its copyright
notices remain applicable.

The generic terminal-profile importer is an original C implementation of the
published Windows Terminal and VS Code settings formats. Format documentation
and ownership boundaries are listed in [Terminal profiles](Terminal-profiles.md).
No donor parser, JSON library, application code or shell executable is bundled
by that importer.

The image rebuild workflow and shared device preview fixture were adapted from
DRSDavidSoft/Ardush MIT-licensed preview tooling. The C terminal replay renderer
is implemented in this repository.

Presentation design references included atomicdeploy/PCController,
atomicdeploy/RayanPBX and the serial shell in atomicdeploy/pu850-esp. These credits
do not imply runtime dependencies or platform support.

Preview fonts and rendering tools have separate licenses. Their pinned revisions,
hashes and license downloads are maintained in tools/previews/dependencies.cmake.
They are not linked into the shell library.

## Consumer maps

- [Ardush serial adapter and board builds](https://github.com/DRSDavidSoft/Ardush/blob/master/ARCHITECTURE.md)
- [PakarController ownership and terminal integration](https://github.com/atomicdeploy/PakarController/blob/update/portable-shell-refresh/docs/SYSTEM-MAPS.md)

PU850 is a prospective adapter migration, not a completed consumer refresh. Preserve its device commands and persistence semantics when integrating; do not substitute desktop demo commands.

[Source disposition audit and migration gates](Source-disposition.md)
