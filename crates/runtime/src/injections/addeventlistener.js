(function () {
  globalThis.addEventListener = function (type, listener, options) {
    console.debug('addEventListener called with type =', type);
    // No-op implementation
  };
})();
