# srvcs-vectoradd

Concern: **vectors: component-wise addition**

Adds two equal-length vectors component by component. This service is an
*orchestrator*: it owns the control flow but delegates every scalar addition to
its primitive dependency.

- Depends on: `srvcs-floatadd`

## Algorithm

Given the request `{"a": [number, ...], "b": [number, ...]}` where `a` and `b`
have equal length, for each index `i`:

```
result[i] = floatadd({"a": a[i], "b": b[i]}).result
```

The `result` is the list of those component sums (a JSON array of `f64`).

```
vectoradd([1, 2], [3, 4]) = [4.0, 6.0]
```

Vectors are read as JSON arrays; each element is passed straight into the
dependency request body. This service does **not** call `srvcs-isnumber`
directly — element-level validation propagates from `srvcs-floatadd` (its
`422`s are forwarded verbatim). The one validation this service owns is the
equal-length requirement: a length mismatch is rejected with `422`.

## API

### `GET /`

Service identity.

```json
{
  "service": "srvcs-vectoradd",
  "concern": "vectors: component-wise addition",
  "depends_on": ["srvcs-floatadd"]
}
```

### `POST /`

Request:

```json
{ "a": [1, 2], "b": [3, 4] }
```

Response `200`:

```json
{ "a": [1, 2], "b": [3, 4], "result": [4.0, 6.0] }
```

Statuses:

- `200` — the component-wise sum.
- `422` — the vectors differ in length, or `srvcs-floatadd` rejected a
  component (forwarded).
- `500` — `srvcs-floatadd` returned a `200` without a usable numeric `result`.
- `503` — `srvcs-floatadd` is unreachable; this service reports itself degraded.

## Configuration

| Variable             | Default                 | Description                   |
| -------------------- | ----------------------- | ----------------------------- |
| `SRVCS_BIND_ADDR`    | `0.0.0.0:8080`          | Listen address.               |
| `SRVCS_FLOATADD_URL` | `http://127.0.0.1:8090` | Base URL of `srvcs-floatadd`. |
| `RUST_LOG`           | `info,tower_http=info`  | Log filter.                   |
| `SRVCS_ENV`          | `development`           | Environment label.            |

## Local checks

```sh
nix flake check -L
nix develop -c sh -euc 'cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test'
nix build .#default -L
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared service
standard and CI workflow.
