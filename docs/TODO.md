# ccmux TODO

## Performance Improvements (Future Work)

### Blocking I/O in Async Context

**Issue:** `Session::read_output()` performs blocking `std::io::Read` on the PTY and blocking filesystem operations (`create_dir_all`, `OpenOptions::open`, `flush`) in async context. This can block the Tokio runtime and stall other connections.

**Location:** `src/server/session.rs:94-141`

**Solution Options:**
1. Move PTY + log writing to a dedicated blocking task/thread using `tokio::task::spawn_blocking`
2. Use non-blocking/async I/O for PTY operations
3. Set PTY file descriptor to non-blocking mode

**Priority:** Medium - affects performance with chatty sessions

---

### Log File Write Overhead

**Issue:** `append_log()` opens and flushes the log file for every output chunk. This becomes significant overhead for chatty sessions.

**Location:** `src/server/session.rs:123-141`

**Solution Options:**
1. Keep a `BufWriter<File>` in the `Session` struct and flush periodically / on shutdown
2. Remove explicit `flush()` on every append unless required
3. Use async file I/O with buffering

**Priority:** Medium - affects performance but not correctness

---

## Completed

- [x] Connection handler isolated from daemon state - Fixed with message passing
- [x] Sessions never started - Fixed by calling `Session::start()`
- [x] Duplicate SessionStatus enum - Consolidated to protocol.rs
- [x] Lockfile race condition - Fixed with atomic `create_new(true)`
- [x] Lockfile symlink attack - Fixed with atomic creation + restrictive permissions
- [x] Stream framing - Fixed with EOF-based reading
- [x] SessionStatusDetail mismatch - Fixed by returning correct type
