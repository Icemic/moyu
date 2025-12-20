/**
 * Stub implementations for certain browser APIs that are not available
 * in the runtime environment.
 */

(function () {
  globalThis.addEventListener = function (type, listener, options) {
    console.debug('addEventListener called with type =', type);
    // No-op implementation
  };
  globalThis.postMessage = function (message, targetOrigin, transfer) {
    console.debug('postMessage called with message =', message);
    // No-op implementation
  };
  globalThis.__moyu_receive_event = function (events) {
    // No-op implementation
  };
})();
