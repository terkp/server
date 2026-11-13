# Handoff: bug audit of the quiz server

Status on 2026-11-13: the audit is finished and a second agent has reviewed it (see "Review results" at the end). **No code has been changed yet.** The next step is to implement the fix plan below.

To continue on another computer, check out this branch, open the folder in Claude Code and say:

> Read HANDOFF.md and implement the fix plan. Take the review results at the end into account. Run `cargo check` when done and tell me what you could not test.

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

Pending: the review agent had not finished when this file was written.
