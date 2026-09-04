# Security policy

## Supported versions

TSG is pre-1.0. Security fixes apply to the latest released minor version.

## Reporting

Do not disclose suspected vulnerabilities in a public issue. Report them
privately to the repository owner with reproduction steps, affected versions,
and impact. Raw graph content and embeddings may contain sensitive source
information and must not be attached unless explicitly requested over a secure
channel.

## Data boundary

TSG performs no network access or telemetry. SQLite databases, WAL files,
database backups, lock files, and USearch sidecars remain local. Callers must
protect all of them as sensitive data. Advisory locking is supported only on
local filesystems; NFS and SMB are outside the security and correctness model.
