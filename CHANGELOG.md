# Change Log

## [Unreleased]
- Initial release

## [0.0.4] - 2023-02-18 - [VSIX]()
### Fix
- Fix the search file content handler name

## [0.0.3] - 2023-02-18 - [VSIX]()
### Added
- Icon!!!!!

## [0.0.2] - 2023-01-18 - [VSIX]()
### Added
- New dependencies, yay. :(
- Now all the default commands are going through [binocular-cli](https://github.com/jpcrs/binocular-cli). This creates a new dependency to yet another tool, but should make things more reliable in other systems (Not having to rely sed magic, yay, sanity back)
### Changed
- All the command handlers are different
- There is no more tmp file writing/reading for every command, binocular-cli handles opening files/folders on code.
### Removed
- Unfortunately lost the hability of closing things inside vscode, since the extension itself it not executing any command anymore. Someday I'll take this serious and do it in a reliable way.

## [0.0.1] - 20XX-XX-XX - [VSIX]()
### Added
- Initial Release, including:
    - ?