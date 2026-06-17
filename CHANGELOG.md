# Changelog

All notable local changes to this Hitide fork are recorded here.

## 0.17.1 - 2026-06-11

### Release Follow-up - 2026-06-17

- Added a short backend connection-pool idle timeout so Hitide reconnects
  quickly after a Lotide backend restart instead of waiting for a full page
  request timeout on stale pooled sockets.

### Release Hygiene - 2026-06-16

- Added `cargo-deny` policy files and verified advisories, crate bans, and
  source policies for the release tree.
- Removed stale unused dependencies and added narrow `cargo-machete`
  exceptions where generated icon code makes usage non-obvious.
- Cleaned vendored render and Trout metadata, readmes, doc comments, and
  doctests so packaged documentation examples compile cleanly.
- Verified the release tree with `cargo fmt --all --check`, strict workspace
  Clippy with warnings denied, `cargo audit`, `cargo deny`, `cargo machete`,
  rustdoc with warnings denied, and full workspace tests including doctests.

### Added

- Added the dark theme with red accents while keeping Hitide's simple
  server-rendered HTML model.
- Added configurable site name, logo, favicon, and custom CSS support through
  Lotide-backed site settings.
- Added themed error pages and clearer service failure messages, including
  `No backend` when Hitide cannot reach Lotide.
- Added federation status badges for local posts, comments, likes, follows, and
  unfollows where the backend exposes delivery checkpoints.
- Added personal follow notifications to the user-facing notification views.
- Added user avatar/profile image controls for local accounts.
- Added community list search, platform filters, counts, sorting, "mine" and
  "everything" scopes, direct unfollow controls, recent-post hints, and numbered
  pagination.
- Added admin federation health views with host profiles, recent federation
  events, failing-host summaries, replayable task controls, cleanup controls,
  site settings, logo upload, and custom CSS upload.
- Added configurable bind address support so Hitide can listen on an external
  interface when a deployment needs that.
- Added Debian build and install scripts for project-local deployment. The
  broader workspace also contains MSYS2-oriented proof helpers.

### Changed

- Updated the project to Rust 2024 and bumped the local release version to
  `0.17.1`.
- Modernized the HTTP stack to Hyper 1, `http` 1, `headers` 0.4,
  `hyper-util`, and `http-body-util`.
- Reworked backend request handling so connection failures are reported as
  operator-usable frontend errors instead of generic internal server errors.
- Reworked backend instance metadata loading so older or partially migrated
  Lotide backends fall back to default page chrome instead of breaking ordinary
  frontend pages.
- Reworked backend upload requests to use a longer streaming timeout while
  keeping normal page API requests on a shorter timeout.
- Reworked community pages to reflect server, community, and user interaction
  status without hiding actions on detail pages inconsistently.
- Reworked admin federation diagnostics so long multiline remote errors are
  summarized instead of breaking the layout.
- Reworked templates to tolerate malformed or missing backend fields without
  turning ordinary bad remote data into page-wide failures.
- Reworked install documentation to match the current service scripts and
  runtime settings.

### Fixed

- Fixed several stale frontend deployments where backend features existed but
  the UI did not expose them.
- Fixed generic internal server error screens for missing backend connectivity.
- Fixed admin page formatting issues caused by escaped carriage returns, raw
  remote HTML, and very long federation error strings.
- Fixed user pages that failed when backend federation fields were absent,
  malformed, or newly added by migration.
- Fixed community list search and filtering so hostnames and current scope are
  respected.
- Fixed mobile and narrow-width layout regressions introduced by the larger
  community and admin pages.
- Removed a stale backend deployment script from the Hitide tree.

### Tests

- Added route and render tests for backend failure pages.
- Added render tests for federation status, community listing, admin
  diagnostics, malformed timestamps, and missing optional backend data.
- Ran stricter Clippy passes with warnings denied after the HTTP modernization.

<!-- end of CHANGELOG.md -->
