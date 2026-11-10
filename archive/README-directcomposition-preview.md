# DirectComposition preview source restoration

The [complete archived source](./directcomposition-preview/) is the exact Git
tree from [`archive/directcomposition-preview`](https://github.com/ToghrolTP/pealayer/tree/1b6d89d80af69ffa6d9f53d9f2d16f4d4ad9ed91).
The archive head is a parent of this branch. The snapshot includes the original
build files, source, assets, tests and documentation.

The archive contains the optional D3D11 presentation path, DWM thumbnail
handoff, offscreen video host, popup fallback and shared OSD cards. It also
contains window placement, subtitle styling, hold-to-fast-forward timing and
direct PWM envelopes. The active app uses the current root source. The nested
snapshot has no production build or runtime entry point.

- [Renderer and preview architecture](./directcomposition-preview/docs/EXPERIMENT-D3D11-DIRECTCOMPOSITION.md)
- [Authored PWM design and safety limits](./directcomposition-preview/docs/EFFECT-CUE-TIMING.md)
- [Windows preview integration](https://github.com/ToghrolTP/pealayer/issues/64)
- [Adapter and decoder validation](https://github.com/ToghrolTP/pealayer/issues/65)
- [Adjacent-frame behavior](https://github.com/ToghrolTP/pealayer/issues/66)
- [Hardware authoring and timing acceptance](https://github.com/ToghrolTP/pealayer/issues/40)

To verify the snapshot, compare the tree IDs from
`git rev-parse HEAD:archive/directcomposition-preview` and
`git rev-parse "archive/directcomposition-preview^{tree}"`.
