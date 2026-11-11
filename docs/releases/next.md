# 🚧 Next Pealayer release

The current milestone is [v0.12.0-alpha.2](v0.12.0-alpha.2.md). See the [version index](README.md) for published checkpoints and [current acceptance](../verification/CURRENT-ACCEPTANCE.md) for remaining operational work.

Publish on major integrated checkpoints after the relevant runtime checks and platform matrix pass. Group smaller fixes into the next checkpoint. Align the package version, add [curated notes](TEMPLATE.md), and tag the verified source with a new semantic version. Verify the published archives, checksums and provenance, then update the release ledger.

If GitHub refuses the workflow token after attestation, download the run's `attested-release-assets` artifact. Verify `SHA256SUMS.txt` and the asset attestations against the existing tag and exact source commit. A release maintainer can publish those same assets with `gh release create --verify-tag`, using the attached `RELEASE-NOTES.md` for the body.
