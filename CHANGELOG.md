# Changelog

All notable local changes to this Hitide fork are recorded here.

## 0.17.0 - 2026-06-10

### Added

- Added the dark red theme for the existing server-rendered pages.
- Added site name, logo, favicon, and custom CSS settings from Lotide.
- Added error pages for backend failures, including
  `No backend` when Hitide cannot reach Lotide.
- Added federation status badges for local posts, comments, likes, follows, and
  unfollows when the backend returns delivery checkpoints.
- Added follow notifications to the notification views.
- Added user avatar/profile image controls for local accounts.
- Added community list search, platform filters, counts, sorting, "mine" and
  "everything" scopes, unfollow buttons, recent-post hints, and numbered
  pagination.
- Added admin federation views for host profiles, recent events, failing hosts,
  retryable tasks, cleanup settings, site settings, logo upload, and custom CSS
  upload.
- Added configurable bind address support for deployments with external reverse
  proxies.
- Added Debian and MSYS2 build scripts.

### Changed

- Updated the project to Rust 2024 and set the local release version to
  `0.17.0`.
- Updated the HTTP stack to Hyper 1, `http` 1, `headers` 0.4,
  `hyper-util`, and `http-body-util`.
- Changed backend request handling to show useful frontend errors for connection
  failures.
- Changed community pages to show server, community, and user interaction status
  consistently.
- Changed admin federation diagnostics to summarize long multiline remote
  errors.
- Changed templates so missing or malformed backend fields do not break whole
  pages.
- Updated install documentation for the current service scripts and runtime
  settings.

### Fixed

- Fixed pages where backend features existed but the UI did not expose them.
- Fixed internal server error screens when the backend was unavailable.
- Fixed admin page formatting issues caused by escaped carriage returns, raw
  remote HTML, and very long federation error strings.
- Fixed user pages that failed when backend federation fields were absent,
  malformed, or newly added by migration.
- Fixed community list search and filtering so hostnames and current scope are
  respected.
- Fixed narrow viewport layout issues in the community and admin pages.

### Tests

- Added route and render tests for backend failure pages.
- Added render tests for federation status, community listing, admin
  diagnostics, malformed timestamps, and missing optional backend data.
- Ran stricter Clippy passes with warnings denied after the HTTP modernization.

<!-- end of CHANGELOG.md -->
