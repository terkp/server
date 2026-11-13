// Helpers shared by the player pages, the display and the admin page.
//
// Everything lives in the single global `Quiz`, because the page scripts
// declare their own top-level names (letters, current_answer, ...) and a
// second declaration of the same name would stop the later script from loading.
//
// How the pages stay in sync: the events from /events are only hints that
// something changed. The truth is fetched from the server (createSync), and
// the pages fetch it again whenever they may have missed something: when the
// event connection opens, when the page becomes visible, and periodically.
window.Quiz = (function () {
  "use strict";

  var RELOAD_KEY = "quiz_reload";
  // Minimum time between two automatic reloads for the same question version.
  var RELOAD_THROTTLE_MS = 10000;

  function escapeHtml(value) {
    return String(value).replace(/[&<>"']/g, function (character) {
      return {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      }[character];
    });
  }

  // fetch() with a timeout that also covers reading the body.
  // Resolves to { ok, status, text } for every HTTP response, including error
  // statuses. Rejects if the server cannot be reached or does not answer in time.
  function fetchWithTimeout(url, options, timeoutMs) {
    var fetchOptions = Object.assign({}, options || {});
    var timer = null;
    if (typeof AbortController !== "undefined") {
      var controller = new AbortController();
      fetchOptions.signal = controller.signal;
      timer = setTimeout(function () {
        controller.abort();
      }, timeoutMs || 8000);
    }
    function stopTimer() {
      if (timer !== null) {
        clearTimeout(timer);
      }
    }
    return fetch(url, fetchOptions)
      .then(function (response) {
        return response.text().then(function (text) {
          return { ok: response.ok, status: response.status, text: text };
        });
      })
      .then(
        function (result) {
          stopTimer();
          return result;
        },
        function (error) {
          stopTimer();
          throw error;
        }
      );
  }

  function fetchJson(url, options, timeoutMs) {
    return fetchWithTimeout(url, options, timeoutMs).then(function (result) {
      if (!result.ok) {
        throw new Error(url + " answered with status " + result.status);
      }
      return JSON.parse(result.text);
    });
  }

  // The state of the current question, plus the stored answer of `groupName`
  // (pass "" on pages that do not belong to a group).
  function fetchState(groupName) {
    return fetchJson(
      "/questions/state",
      {
        method: "POST",
        // The explicit type matters: a body without one is not text/plain.
        headers: { "Content-Type": "text/plain" },
        body: groupName || "",
        cache: "no-store",
      },
      5000
    );
  }

  function fetchGroups() {
    return fetchJson("/groups/get", { cache: "no-store" }, 5000);
  }

  // Keeps a piece of server state up to date. Returns a function; calling it
  // requests a refresh: `load()` is fetched and the result handed to `apply`.
  //
  // - Only one request runs at a time, so results are applied in order.
  // - If a refresh is requested while a request is running, that request's
  //   result may already be outdated. It is dropped and fetched again, so an
  //   old answer never overwrites what a newer event already put on the page.
  //   With `applyOutdated` the outdated result is applied anyway before
  //   fetching again (for data that changes in bursts, like the group list).
  // - Failed requests are retried with a growing delay while the page is
  //   visible. A hidden page stops and relies on the next request.
  //
  // `refresh.whenApplied()` requests a refresh and resolves with the result
  // once it was applied.
  function createSync(load, apply, applyOutdated) {
    var requested = 0;
    var running = false;
    var waiters = [];

    function deliver(result) {
      try {
        apply(result);
      } catch (error) {
        console.error(error);
      }
    }

    function attempt(delay) {
      var generation = requested;
      load().then(
        function (result) {
          if (generation !== requested) {
            if (applyOutdated) {
              deliver(result);
            }
            attempt(500);
            return;
          }
          running = false;
          deliver(result);
          var resolvers = waiters;
          waiters = [];
          resolvers.forEach(function (resolve) {
            resolve(result);
          });
        },
        function () {
          if (document.visibilityState === "hidden") {
            running = false;
            return;
          }
          setTimeout(function () {
            attempt(Math.min(delay * 2, 5000));
          }, delay);
        }
      );
    }

    function refresh() {
      requested += 1;
      if (!running) {
        running = true;
        attempt(500);
      }
    }

    refresh.whenApplied = function () {
      return new Promise(function (resolve) {
        waiters.push(resolve);
        refresh();
      });
    };

    return refresh;
  }

  // Opens the event stream. `onMessage(data)` is called for every event,
  // `onOpen(isReconnect)` every time a connection is established, which is
  // the moment to fetch the state, because events may have been missed.
  //
  // The browser reconnects by itself after a lost connection. It gives up
  // (readyState CLOSED) when the server answers with an error, for example
  // while it is restarting; then a new EventSource is created after 2 s.
  //
  // Returns { reconnect, lastActivity }. `reconnect()` replaces the connection
  // with a new one; use it when the connection may be dead without the browser
  // noticing (after the phone slept or changed the access point).
  // `lastActivity()` is the time of the last open or message.
  function connectEvents(onMessage, onOpen) {
    var source = null;
    var retryTimer = null;
    var openCount = 0;
    var activity = Date.now();

    if (typeof EventSource === "undefined") {
      alert("Whoops! Your browser doesn't receive server-sent events.");
      return {
        reconnect: function () {},
        lastActivity: function () {
          return Date.now();
        },
      };
    }

    function open() {
      if (retryTimer !== null) {
        clearTimeout(retryTimer);
        retryTimer = null;
      }
      if (source !== null) {
        source.onopen = null;
        source.onmessage = null;
        source.onerror = null;
        source.close();
      }
      var current = new EventSource("/events");
      source = current;
      current.onopen = function () {
        activity = Date.now();
        openCount += 1;
        if (onOpen) {
          onOpen(openCount > 1);
        }
      };
      current.onmessage = function (event) {
        activity = Date.now();
        onMessage(event.data);
      };
      current.onerror = function () {
        if (current.readyState === EventSource.CLOSED && retryTimer === null) {
          retryTimer = setTimeout(open, 2000);
        }
      };
    }

    open();
    return {
      reconnect: open,
      lastActivity: function () {
        return activity;
      },
    };
  }

  var navigating = false;

  // Reloads the page (`url` null) or goes to `url`, but only once the server
  // answers. Navigating while it is unreachable would leave the browser on an
  // error page where no script runs, and the device would be stuck.
  // Pass `serverJustAnswered` if a request has just succeeded; then the check
  // is skipped. Calling this again while a navigation is pending does nothing.
  function navigateWhenReachable(url, serverJustAnswered) {
    if (navigating) {
      return;
    }
    navigating = true;

    function go() {
      if (url === null || url === undefined) {
        window.location.reload();
      } else {
        window.location.href = url;
      }
    }

    function probe() {
      fetchWithTimeout("/ping", { cache: "no-store" }, 3000).then(
        function (result) {
          if (result.ok) {
            go();
          } else {
            setTimeout(probe, 1000);
          }
        },
        function () {
          setTimeout(probe, 1000);
        }
      );
    }

    if (serverJustAnswered) {
      go();
    } else {
      probe();
    }
  }

  // A page that comes back from the browser's back/forward cache continues
  // with its old script state; a navigation that was pending then is over.
  window.addEventListener("pageshow", function (event) {
    if (event.persisted) {
      navigating = false;
    }
  });

  // The question version the server rendered this page with.
  function pageVersion() {
    var version = document.body.dataset.questionVersion;
    return version === undefined ? null : version;
  }

  // Whether the page shows the question that `state` (from fetchState)
  // belongs to. If not, the page is reloaded and false is returned.
  //
  // The comparison is done on strings: the page's version is a string, the
  // one in the state a number. As a safety net against reload loops, the page
  // is reloaded at most once per version within RELOAD_THROTTLE_MS.
  function pageIsCurrent(state) {
    var shown = pageVersion();
    var wanted = String(state.questionVersion);
    if (shown === null || shown === wanted) {
      return true;
    }

    // Per page, because pages in one tab (frames) share the sessionStorage.
    var key = RELOAD_KEY + ":" + window.location.pathname;
    var now = Date.now();
    try {
      var last = JSON.parse(sessionStorage.getItem(key));
      if (
        last !== null &&
        last.version === wanted &&
        now - last.time < RELOAD_THROTTLE_MS
      ) {
        return false;
      }
      sessionStorage.setItem(
        key,
        JSON.stringify({ version: wanted, time: now })
      );
    } catch (error) {
      // No usable sessionStorage (private mode): reload without the safety net.
    }
    navigateWhenReachable(null, true);
    return false;
  }

  return {
    escapeHtml: escapeHtml,
    fetchWithTimeout: fetchWithTimeout,
    fetchJson: fetchJson,
    fetchState: fetchState,
    fetchGroups: fetchGroups,
    createSync: createSync,
    connectEvents: connectEvents,
    navigateWhenReachable: navigateWhenReachable,
    pageVersion: pageVersion,
    pageIsCurrent: pageIsCurrent,
  };
})();
