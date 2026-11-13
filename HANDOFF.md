# Handoff: bug audit of the quiz server

Status on 2026-11-13: the audit is finished. An independent review of it by a second agent was started; whether it finished is recorded under "Review results" at the end. **No code has been changed yet.** The next step is to implement the fix plan below.

To continue on another computer, check out this branch, open the folder in Claude Code and say:

> Read HANDOFF.md. If the "Review results" section says the review did not finish, first have a separate agent verify the findings and the fix plan against the code (read-only), as that section describes. Then implement the fix plan, taking the review results into account. Run `cargo check` when done and tell me what you could not test.

The baseline compiles (`cargo check` passes on commit 40d8e38, Rocket 0.5.1, rocket_dyn_templates 0.1.0).

Two files were untracked on the first computer and are not part of this branch: `pictures/n1_quiz_smd.jpg` and `res/Fragen_Quiz_SMD.txt`. Copy them yourself if you need them.

## Findings

### Network

| ID | Where | Problem |
|---|---|---|
| N1 | `src/server_data.rs` `EventBuffer::push` (~353) | Adds a semaphore permit even when the queue is full. Permits then exceed queued events and `pop()` panics on `self.0.pop().unwrap()` inside the SSE stream. |
| N2 | `src/main.rs` `events` (capacity 4), `send_event` in `server_data.rs` | A client whose 4-slot queue overflows (many groups answering at once) is removed from the buffer map although it is still connected. It then stops receiving events. |
| N3 | `frontend/ui/index.js`, `frontend/display/index.js` | Nothing resyncs after an EventSource reconnect. A phone that missed the "question" event keeps the old question, and its answer is stored against the new one. Events between page render and EventSource subscription are lost too. |
| N4 | same files | `location.reload()` while offline leaves the browser on an error page with no script running. The device is stuck until someone reloads by hand. |
| N5 | `frontend/ui/normal.js`, `estimate.js`, `sort.js`, `zuordnen.js` | Answer submission has no error or timeout handling, so a failed send gives no feedback. `estimate.js` also shows "Deine Antwort" for any status that is not "not found", including 500. |
| N6 | same files plus `ui/index.js` | The lock flags live in `localStorage` and are only refreshed when the state request succeeds. If it fails after a page load, stale `"true"` values from the previous question keep the send button disabled. |
| N7 | `frontend/admin/index.js` (showAnswers, toggleLeaderboard, setScore, addScore, deleteGroup), `src/group.rs` | `fetch(...).await;` is a no-op property read, so `updateGroups()` runs before the request finishes. `set_score` and `add_score` never send an `UpdateGroups` event. The admin table shows stale scores. |
| N8 | `frontend/ui/login.js` | Stores the group name and redirects even when the request failed with a network error (status 0). The group then does not exist on the server. |
| N9 | `Rocket.toml`, `src/questions.rs` `load_questions`, `admin/index.js` `loadQuestions` | Rocket's default `string` limit is 8 KiB, so a larger question file is rejected. The admin page ignores the response, so neither that nor a parse error (400) is reported. It throws if no file is selected. |
| N10 | `src/questions.rs` `record_results_if_needed` | Check-then-act across an await. Two concurrent `/questions/show_solution` requests (double click, browser GET retry) can both award points. |

### General

| ID | Where | Problem |
|---|---|---|
| G1 | `src/main.rs` `show_ui`, `src/display.rs` `show_display` | `questions[current_question]` is indexed after only an `is_empty()` check. `/questions/next` past the last question, or `/questions/set` with a number that is too high, makes every page render panic (500). |
| G2 | `src/questions.rs` `show_answers`, `src/group.rs` `set_answer` | `block_answer` is set but never read, so answers are accepted after they were revealed. It is also set after the event is sent. |
| G3 | `src/server_data.rs` `Answer::try_parse_answer`, `ServerData::results` | Estimate answers accept `NaN` and `inf`. `partial_cmp(...).unwrap()` then panics in `results()`, so "Lösung anzeigen" returns 500 until that group is deleted. Normal answers accept any index. |
| G4 | `ServerData::results` | `add_group_points(...)?` returns early if a group was deleted meanwhile. Points are partly awarded and a retry double-counts. |
| G5 | `frontend/display/index.js` `updateGroups`, `frontend/admin/index.js` `updateGroups` and `updateQuestion` | Group names (chosen by players) are inserted with `innerHTML` unescaped: stored XSS on the display and in the admin panel. Server-rendered Tera templates are believed to be auto-escaped. |
| G6 | `src/display.rs` `show_leaderboard` | Iterates the DashMap in arbitrary order, so the leaderboard is not sorted. |
| G7 | `src/group.rs` `new_group` | Group names are not validated on the server (empty, whitespace only, any length, not trimmed). |
| G8 | `frontend/ui/estimate.html.tera` | `onchange` enables the send button only after the input loses focus, so the first tap on the button is swallowed. |
| G9 | `src/questions.rs` `load_questions`, `Question::from_str` | Loading questions does not clear group answers. The estimate solution is not trimmed and accepts `NaN`/`inf`. |
| G10 | `frontend/ui/sort.js`, `src/group.rs` `set_answer` | "Deine Antwort" is built from `current_answer` at response time. `set_answer` uses `dbg!` and returns 500 for a client parse error. |
| G11 | `src/questions.rs` `current_question` | Locks `questions` twice (length check, then index). |

### Known, deliberately not fixed

These are design issues, not bugs. Fixing them would be a redesign.

- Admin endpoints have no authentication (`/admin_billig` is only hidden).
- `GET /questions/current` and the hidden `#solution` element give the solution to any player who looks.
- `GET /groups/get` exposes every group's answer before the reveal.
- State-changing endpoints are `GET` (`/questions/next`, `/questions/show_solution`, ...).
- CORS sends `Access-Control-Allow-Origin: *` together with `Allow-Credentials: true`.

## Fix plan

### Server

**P1. Events over a broadcast channel.** Replace `EventBuffer`, the `client_event_buffers` map and the crossbeam dependency with one `rocket::tokio::sync::broadcast::Sender<UpdateEvent>` in `ServerData` (capacity about 256).

- `/events` calls `subscribe()` and loops with `select!` over `rx.recv()` and `Shutdown`.
- On `RecvError::Lagged` it yields a data event `"resync"`. On `Closed` it breaks.
- The first yield is `Event::retry(Duration::from_secs(1))` so browsers reconnect quickly.
- `send_event` becomes `let _ = server_data.events.send(event);`.
- `ServerData` needs a manual `Default` impl.

**P2. Question version.** Add `question_version: AtomicUsize` to `ServerData`, seeded with a random `u32` at startup so a server restart is also seen as a change.

- One helper performs every question change (load, next, set). While holding the `questions` mutex it stores the new index, clears `block_answer`, clears all group answers, resets `question_state` and increments the version.
- `show_ui` and `show_display` read index and version while holding the same mutex, so a page never carries a version newer than its content.
- The templates put the version on the page: `<body data-question-version="{{ question_version }}">`. Every render path, including the waiting page, must pass it.
- `next` is clamped to `len`. Index equal to `len` means "quiz finished" and renders the existing waiting template through `questions.get(idx)`. If already at `len`, do nothing.
- `set` with an index of `len` or more returns 400.

**P3. State endpoint.** `POST /questions/state` always returns 200 with `{answer, questionState, questionVersion, groupExists}`. Today it returns 404 when there is no current question.

**P4. `set_answer`.**

- Accept an optional `question_version` in the JSON. Return 409 if it is present and differs from the current version.
- Return 403 if `block_answer` is set.
- Return 400 for an answer that cannot be parsed. Reject non-finite estimates and normal answers of 4 or more.
- Remove `dbg!`.
- `show_answers` sets `block_answer` before it sends the event.

**P5. Results.** `record_results_if_needed` does `results_recorded.swap(true)`. If it was already true, return. If `results()` fails, store false again. `results()` skips groups that no longer exist and sorts with `total_cmp`.

**P6. Smaller server fixes.**

- `set_score` and `add_score` send `UpdateGroups`.
- Group names are trimmed, must not be empty and may be at most 40 characters.
- The leaderboard is sorted by score descending, then by name.
- `load_questions` returns 400 with a text of the form `line N: reason`, strips a UTF-8 BOM and trims the estimate solution.
- `current_question` locks once and uses `get`.
- `Rocket.toml` gets `[default.limits]` with `string = "1 MiB"`.

### Frontend

**P7. Shared `frontend/res/common.js`**, served at `/res/common.js` by the existing `files` route and included by the ui index, display index, display score and admin templates. It contains:

- `connectEvents(onMessage, onOpen)`: creates the EventSource, calls `onOpen` on every open, and recreates the EventSource after 2 s if `readyState` is `CLOSED` in `onerror`.
- `fetchWithTimeout`: fetch with an `AbortController` timeout.
- `navigateWhenReachable(url)`: polls the server with a cheap request and reloads or navigates only once it answers, retrying every second. `null` means reload.
- `escapeHtml`.

**P8. `ui/index.js`.**

- Replace the `localStorage` lock flags with a global `quizLocked`. The per-type scripts read `window.quizLocked`; they load before `index.js`, so `undefined` must count as unlocked.
- `resync()` fetches `/questions/state` with retry.
  - If `questionVersion` differs from the page's version, call `navigateWhenReachable(null)`.
  - If the group no longer exists, remove `group_name` and go to `/login`.
  - Otherwise apply the lock and solution state and show the answer the group already submitted.
- Call `resync()` on page load, on every EventSource open, when the page becomes visible again, and on `"question"` and `"resync"` events.
- One shared `submitAnswer(type, answerString, displayText, onSuccess)` with a timeout, an alert on failure, and handling for 403, 409 and "not found". The four per-type scripts use it and pass the display text captured at submit time.
- Set the group label with `textContent`.
- `estimate.html.tera` uses `oninput`.

**P9. Display and admin.**

- `display/index.js`: build the group list with `textContent`, ignore `/groups/get` responses that arrive out of order, use `navigateWhenReachable` for every reload and navigation, reload when the EventSource reopens (not on the first open), and guard against a missing `#solution` element.
- `display/score.js`: use the same helpers and reload on `"groups"` events so scores stay current.
- `admin/index.js`: replace `.await` with `.then`, escape HTML, report load errors and a missing file, refresh everything on EventSource open, and catch failed fetches.

**P10. `login.js`.** On status 0 show an error and stay on the page. Trim the name. On a 2xx or a 422 "already exists" (joining an existing group, which is today's behaviour) store the name and continue.

### After implementing

- Run `cargo check` and `cargo build`.
- Test by hand with two browser tabs (`/` and `/display`) plus `/admin_billig`: load `res/fragen.txt`, answer, reveal, next question, go past the last question, stop and restart the server while the tabs are open, and switch a tab's network off and on in the browser dev tools.
- Remove this file once the work is merged.

## Review results

**The review finished.** No finding was refuted. Three were corrected (N2, N4, N9), the fix plan has several traps, and the reviewer found bugs the audit missed (A1 to A6). The reviewer read the code and the Rocket 0.5.1, rocket_dyn_templates 0.1.0 and tera 1.20.1 sources but did not run the server. Where the review and the fix plan above disagree, the review wins.

For reference, the reviewer was asked to work read-only and report:

1. A verdict for each finding ID (confirmed, refuted or partly correct) with file and line evidence.
2. Problems in the fix plan that would break existing behaviour. Points worth checking:
   - Whether Rocket matches `format = "text/plain"` on the state request when the body is empty or null.
   - Whether Tera fails on a `question_version` that one render path forgets to pass.
   - Script load order: the per-type scripts sit inside the `answers` block, `ui/index.js` loads at the end of the body.
   - Whether `EventStream!` works with a broadcast receiver and `Shutdown` in one `select!`, and whether `Event::retry` and `rocket::tokio::sync::broadcast` exist in Rocket 0.5.1.
   - Whether Tera really auto-escapes the `.html.tera` templates (finding G5 assumes it does).
   - Races in the question-version scheme.
3. Real bugs the audit missed, network-related first.

### Corrections to the findings

- **N2** is not permanent. The dropped client drains its 4 events, hits the N1 panic, the connection ends, and EventSource reconnects after 3 to 5 s. The real loss is the overflowed event plus everything in the reconnect gap (N3).
- **N4** is rare: the realistic triggers are a server restart or a 500 from G1.
- **N6**: a missing key (first load with a failed state request) also blocks the button. `display/index.js` writes the same `localStorage` keys.
- **N7**: only set and add score stay stale. The other `.await` uses are harmless.
- **N9**: exceeding the 8 KiB limit returns **400**, not 413, so it looks like a parse error. Current files are about 2.3 KB.
- **G1**: the last line of `res/Fragen_Quiz_SMD.txt` is a dummy "quiz over" question, apparently a workaround for this crash.
- **G3**: only `NaN` panics, and only with at least two estimate answers. It needs a hand-made request. It clears when that group submits again.
- **G4, G11, N10**: confirmed but need true concurrency, so they are unlikely.
- **G5**: the Tera auto-escape assumption is correct (`rocket_dyn_templates-0.1.0/src/tera_templates.rs:16-17`).
- **G7**: phone keyboards append a trailing space, so "Team" and "Team " become two groups.
- **G10** is wider: `normal.js:42-43` and `estimate.js:47` also read the answer at response time. Tap A, send, tap B, and a slow response shows "Deine Antwort: B" while the server holds A.

### Bugs the audit missed

- **A1. The display's state restore is broken.** `display/index.js:72` sends `localStorage.getItem("group_name")`, which is null on the projector browser. The request then has no Content-Type, the route with `format = "text/plain"` does not match, and the result is 404. `applyQuestionState` never runs. Estimate pictures that should start inline never appear, and a display reload mid-question loses shown answers, solution and picture mode. It works only in a browser that once logged in as a player.
- **A2. A 10.6 MB picture goes to every phone, uncached.** `pictures/n1_quiz_smd.jpg` is used with `pic -bu`. `NamedFile` sends no ETag, Last-Modified or Cache-Control, so every load of that question downloads it again: about 420 MB for 40 phones. Shrink the image to a few hundred KB and send `Cache-Control: max-age=...` for `/pictures` and static files. More reloads from the fix plan make this worse.
- **A3. "Nächste Frage" is relative.** `fetch_add` behind a GET: a double click or a browser retry skips a question. Send the expected current index or the target index.
- **A4. The leaderboard toggle has no server state.** `toggle_leaderboard` is only an event. A display that misses it is on the wrong page and the admin button is inverted from then on. Store a `leaderboard_shown` flag and return it from `/questions/state`.
- **A5. Console logging load.** Two `info!` lines per event per client, plus `println!`, `dbg!` and Rocket's request lines. In a Windows console a click in the window (QuickEdit) blocks stdout and with it the workers. Set `log_level = "critical"` in `Rocket.toml` for the event.
- **A6. The admin page needs the internet.** `frontend/admin/style.css:1` has an active Google Fonts `@import`, which blocks rendering on a LAN without uplink until it fails.

### Problems in the fix plan

API checks that passed: `Event::retry` exists, `rocket::tokio::sync::broadcast` is reachable, `select!` with `yield` inside `EventStream!` compiles, `/res/common.js` collides with nothing, and `[default.limits] string = "1 MiB"` is the right key.

What would break:

1. **Deadlock in the P2 helper.** `reset_question_state_for_current_question` locks `questions` itself (`server_data.rs:208`). Called while the helper holds that mutex, it hangs forever. Pass the guard or slice in.
2. **State request without Content-Type returns 404.** Every state request must send a string body (`""` is enough) with an explicit `Content-Type: text/plain`, or the route should drop `format`. This also fixes A1.
3. **A missing `question_version` in any render path is a 500.** Tera errors on an undefined variable. The waiting paths (`main.rs:200`, `display.rs:37-40`) and the "finished" path must pass the real version. A `default("")` would cause a reload loop.
4. **Reload loop from a type mismatch.** `dataset.questionVersion` is a string and the JSON value is a number, so `!==` is always true. Compare as strings, and allow at most one automatic reload per version (track it in `sessionStorage`).
5. **`window.quizLocked` is undefined if declared with `let` or `const`.** Assign `window.quizLocked = ...` explicitly.
6. **Script order and name clashes.**
   - Per-type scripts run before `#buttonSend`, `#currentAnswer` and `index.js` exist. `zuordnen.js:74` already throws at load today. They must not touch the DOM or shared helpers at top level.
   - `common.js` belongs in `<head>` and must be referenced as `/res/common.js` (absolute), because the score page uses relative URLs.
   - `common.js` must not declare `letters`, `current_answer` or `eSource` at top level; a clash is a SyntaxError that kills the later script. Use one namespace object.
7. **The version check in `set_answer` is racy** unless it holds the `questions` mutex across the version check, the `block_answer` check and the store. `/questions/state` and `show_answers` should use the same mutex.
8. **422 is overloaded.** P10 treats 422 as "duplicate, so join". Use 400 for name validation, or a machine-readable error code. Rocket itself returns 422 for JSON that fails to deserialise.
9. **Trim mismatch can cause a login loop.** JS and Rust `trim()` differ, and old `localStorage` may hold untrimmed names. Have `/groups/new` return the canonical name for the client to store, and normalise the same way in the state and `set_answer` lookups. Count the 40 limit in `chars()`.
10. **Subscribe outside the stream.** Call `subscribe()` in the handler body before `EventStream!`, otherwise a gap reopens between `onopen` and the subscription. Send events only after the state is changed.
11. **`resync()` must be single-flight and monotonic.** It has five triggers. A late older response must not unlock a locked UI.
12. **`navigateWhenReachable` must be idempotent**, or several triggers start parallel reloads. When the state fetch has just shown a new version, that fetch already proves the server is reachable; skip the extra probe.
13. **Do not blindly reload the display on reconnect.** Use the same version-compare resync as the phones, otherwise every WiFi blip blanks the projector.
14. **Do not reload `score.js` on every "groups" event.** It fires for every answer. Re-render from `/groups/get` in place, or debounce.
15. **Leaderboard state**: see A4.
16. **Dead-but-open EventSource is not covered.** After phone sleep or an access-point change the connection can stay `OPEN` with no `onerror`. Add a periodic `resync()` while the page is visible (every 10 to 15 s) and recreate the EventSource on `visibilitychange`.
17. **Out-of-order submissions.** A timed-out request A can land after a later B. Disable send while a request is in flight; after a timeout, resync and show the server's answer before allowing a resend.
18. **Smaller points.**
    - `goToQuestion` in `admin/index.js` ignores the status, so the new 400 must be shown.
    - `readAsText` already strips a BOM; the server-side strip is harmless.
    - With `block_answer` enforced, the only way to reopen answering is to set the question again.

### Not-fixed items, reviewer's opinion

None is a must-fix for the network goal. The one worth reconsidering is the hidden `#solution` element together with `GET /questions/current`: once resync exists, the solution text can be delivered in `/questions/state` only when `solutionIsShown`. A shared admin token would be little work but is a trust decision.
