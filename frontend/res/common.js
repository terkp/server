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

  // The server sends a "ping" event every 2 s (EVENT_PING_INTERVAL in
  // src/main.rs). An open event connection that was silent for this long is
  // dead and gets replaced.
  var SILENCE_LIMIT_MS = 6000;

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
  // - Failed requests are retried with a growing delay (at most 5 s) until
  //   one succeeds. A hidden page retries too: nothing else is guaranteed to
  //   start a new request later, and browsers slow down the timers of hidden
  //   pages on their own.
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

  // Opens the event stream and keeps it open while the page is shown.
  // `onMessage(data)` is called for every event, `onOpen(isReconnect)` every
  // time a connection is established, which is the moment to fetch the state,
  // because events may have been missed.
  //
  // A page does not need to fetch its state when it loads: the first open
  // does that, and it comes after the subscription, so no event can fall in
  // between.
  //
  // Three things can go wrong with the connection:
  // - It is lost. The browser notices and reconnects by itself.
  // - The server answers with an error, for example while it is restarting.
  //   The browser then gives up (readyState CLOSED), and a new EventSource is
  //   created after 2 s.
  // - It is dead without the browser noticing (the phone slept or changed the
  //   access point). The server's "ping" events stop arriving then, and after
  //   SILENCE_LIMIT_MS the connection is replaced. Pings are not passed on to
  //   `onMessage`.
  //
  // A page that is not shown must not hold a connection. Browsers allow only
  // 6 connections per server, shared by all tabs and by the pages they keep
  // in the back/forward cache. Six open streams block every other request to
  // the server, page loads included, until one of them is closed.
  // - A page that is left closes its stream (pagehide). If the browser brings
  //   it back from the back/forward cache, a new one is opened (pageshow).
  // - With `closeWhenHidden` the stream is also closed while the page is
  //   hidden (background tab, locked phone) and opened again when it is
  //   shown. The player page uses this. The display and the admin page keep
  //   their stream, because they have to stay current behind other windows.
  function connectEvents(onMessage, onOpen, closeWhenHidden) {
    var source = null;
    var retryTimer = null;
    var openCount = 0;
    var activity = Date.now();
    var pageLeft = false;

    if (typeof EventSource === "undefined") {
      alert("Whoops! Your browser doesn't receive server-sent events.");
      // Without events the page still gets its state once, and from the
      // periodic refresh of the page scripts.
      setTimeout(function () {
        if (onOpen) {
          onOpen(false);
        }
      }, 0);
      return;
    }

    function close() {
      if (retryTimer !== null) {
        clearTimeout(retryTimer);
        retryTimer = null;
      }
      if (source !== null) {
        source.onopen = null;
        source.onmessage = null;
        source.onerror = null;
        source.close();
        source = null;
      }
    }

    function open() {
      close();
      var current = new EventSource("/events");
      source = current;
      activity = Date.now();
      current.onopen = function () {
        activity = Date.now();
        openCount += 1;
        if (onOpen) {
          onOpen(openCount > 1);
        }
      };
      current.onmessage = function (event) {
        activity = Date.now();
        if (event.data !== "ping") {
          onMessage(event.data);
        }
      };
      current.onerror = function () {
        if (current.readyState === EventSource.CLOSED && retryTimer === null) {
          retryTimer = setTimeout(update, 2000);
        }
      };
    }

    // Opens a fresh stream if the page is shown, closes it otherwise.
    // A fresh one also when the page is shown again after being hidden: the
    // old stream may have died meanwhile without the browser noticing.
    function update() {
      var hidden = closeWhenHidden && document.visibilityState === "hidden";
      if (pageLeft || hidden) {
        close();
      } else {
        open();
      }
    }

    window.addEventListener("pagehide", function () {
      pageLeft = true;
      close();
    });
    window.addEventListener("pageshow", function (event) {
      pageLeft = false;
      if (event.persisted) {
        update();
      }
    });
    if (closeWhenHidden) {
      document.addEventListener("visibilitychange", update);
    }

    setInterval(function () {
      if (
        source !== null &&
        source.readyState === EventSource.OPEN &&
        Date.now() - activity > SILENCE_LIMIT_MS
      ) {
        open();
      }
    }, 1000);

    update();
  }

  var navigating = false;

  // Reloads the page (`url` null) or goes to `url`. Calling this again while
  // a navigation is pending does nothing.
  //
  // Only call this right after a request to the server has succeeded.
  // Navigating while the server is unreachable leaves the browser on an error
  // page where no script runs, and the device is stuck until someone reloads
  // it by hand.
  function navigate(url) {
    if (navigating) {
      return;
    }
    navigating = true;
    if (url === null || url === undefined) {
      window.location.reload();
    } else {
      window.location.href = url;
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
    navigate(null);
    return false;
  }

  return {
    fetchWithTimeout: fetchWithTimeout,
    fetchJson: fetchJson,
    fetchState: fetchState,
    fetchGroups: fetchGroups,
    createSync: createSync,
    connectEvents: connectEvents,
    navigate: navigate,
    pageVersion: pageVersion,
    pageIsCurrent: pageIsCurrent,
  };
})();
