/**
 * Location polyfill for QuickJS environment
 * Make webpack-dev-server HMR work without errors
 */

(function () {
  // Parse URL from __moyu_base_url or use default
  const baseUrl = globalThis.__moyu_base_url || 'http://localhost:6020/';

  // Simple URL parser
  function parseUrl(url) {
    // Match: protocol://host:port/pathname?search#hash
    const match = url.match(/^([^:]+):\/\/([^:/]+)(?::(\d+))?(\/[^?#]*)?(\?[^#]*)?(#.*)?$/);

    if (!match) {
      return {
        protocol: 'http:',
        host: 'localhost',
        hostname: 'localhost',
        port: '6020',
        pathname: '/',
        search: '',
        hash: '',
        href: url,
      };
    }

    const [, protocol, hostname, port, pathname, search, hash] = match;

    return {
      protocol: protocol + ':',
      hostname: hostname,
      port: port || '',
      host: port ? `${hostname}:${port}` : hostname,
      pathname: pathname || '/',
      search: search || '',
      hash: hash || '',
      href: url,
    };
  }

  let currentUrl = parseUrl(baseUrl);

  // Location object with getters and setters
  const location = {
    get href() {
      return currentUrl.href;
    },
    set href(value) {
      console.debug('location.href set to', value);
      currentUrl = parseUrl(value);
    },

    get protocol() {
      return currentUrl.protocol;
    },
    set protocol(value) {
      // Silent no-op
    },

    get host() {
      return currentUrl.host;
    },
    set host(value) {
      // Silent no-op
    },

    get hostname() {
      return currentUrl.hostname;
    },
    set hostname(value) {
      // Silent no-op
    },

    get port() {
      return currentUrl.port;
    },
    set port(value) {
      // Silent no-op
    },

    get pathname() {
      return currentUrl.pathname;
    },
    set pathname(value) {
      // Silent no-op
    },

    get search() {
      return currentUrl.search;
    },
    set search(value) {
      // Silent no-op
    },

    get hash() {
      return currentUrl.hash;
    },
    set hash(value) {
      // Silent no-op
    },

    get origin() {
      return `${currentUrl.protocol}//${currentUrl.host}`;
    },

    // Methods (no-op implementations)
    assign: function (url) {
      console.debug('location.assign() called with url =', url);
    },

    replace: function (url) {
      console.debug('location.replace() called with url =', url);
    },

    reload: function (forcedReload) {
      console.debug('location.reload() called with forcedReload =', forcedReload);
    },

    toString: function () {
      return currentUrl.href;
    },
  };

  // Inject into global scope
  globalThis.location = location;
})();
