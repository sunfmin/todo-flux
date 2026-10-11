# afkconvoy config

What [`afkconvoy`](https://github.com/sunfmin/afkconvoy) reads for this repo:
the yaml block below, and nothing else in this file.

```yaml
base_branch: master        # workers cut from it, open their PR against it, land on it
gate_command: ./gate.sh    # the tests, then Flux on the whole crate
concurrency: 2             # claims held at once, a worker each
```

An issue is dispatched while it carries the label `ready-for-agent`; one the
fleet gives up gets `ready-for-human` instead, with the reason in a comment.
