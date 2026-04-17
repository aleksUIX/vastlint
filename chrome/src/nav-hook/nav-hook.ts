/**
 * nav-hook — runs in the MAIN world (world: "MAIN" in manifest.json).
 *
 * Content scripts live in an isolated JS world and cannot patch
 * history.pushState / history.replaceState on the page itself. This tiny
 * script runs in the page's main world at document_start, wraps both history
 * methods, and fires a custom "vastlint:nav" event on window whenever the URL
 * changes. The isolated-world content script listens for that event.
 */

(function () {
  const dispatch = () => window.dispatchEvent(new CustomEvent('vastlint:nav'));

  const orig = {
    push:    history.pushState.bind(history),
    replace: history.replaceState.bind(history),
  };

  history.pushState = function (...args: Parameters<typeof history.pushState>) {
    orig.push(...args);
    dispatch();
  };

  history.replaceState = function (...args: Parameters<typeof history.replaceState>) {
    orig.replace(...args);
    dispatch();
  };

  // popstate covers back/forward button
  window.addEventListener('popstate', dispatch);
})();
