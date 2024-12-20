# Changelog

### Added

### Changed

### Fixed

## [v2.0.0]

### Added
- Basic support for struct and range types
- Proper ndc-spec type representation

### Changed
- Breaking change: `configuration.json` has changed. Configs prior to v2 need to be deleted and re-initialized

### Fixed
- Fixed capabilities to match what the connector is capable of

## [v1.0.0]

- Updated for stable release

### Added

### Changed

### Fixed

## [v0.1.1] - 2024-09-20

- Initial release with support of ndc-spec v0.1.6
  - Support for CLI plugin for Hasura v3 CLI, which allows the CLI to
    introspect the database on demand.
  - The default port was changed from 8100 to 8080.

<!-- end -->

[Unreleased]: https://github.com/hasura/ndc-bigquery/compare/v0.2.0...HEAD
[v0.1.1]: https://github.com/hasura/ndc-bigquery/releases/tag/v0.1.1
