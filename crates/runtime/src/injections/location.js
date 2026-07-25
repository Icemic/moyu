/**
 * Location polyfill for QuickJS environment
 * Provides the URL and location APIs required by development server HMR
 */

(function () {
  // Parse URL from __moyu_base_url or use default
  const baseUrl = globalThis.__moyu_base_url || 'http://localhost:6020/';

  function resolveUrl(url, base) {
    if (/^[^:]+:\/\//.test(url)) return url;

    const baseUrl = parseUrl(base || globalThis.__moyu_base_url || 'http://localhost:6020/');
    if (url.startsWith('/')) return `${baseUrl.origin}${url}`;

    const basePath = baseUrl.pathname.slice(0, baseUrl.pathname.lastIndexOf('/') + 1);
    return `${baseUrl.origin}${basePath}${url}`;
  }

  function parseUrl(url, base) {
    url = resolveUrl(String(url), base ? String(base) : undefined);
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
        origin: 'http://localhost:6020',
      };
    }

    const [, protocol, hostname, port, pathname, search, hash] = match;

    const origin = `${protocol}://${port ? `${hostname}:${port}` : hostname}`;
    return {
      protocol: protocol + ':',
      hostname: hostname,
      port: port || '',
      host: port ? `${hostname}:${port}` : hostname,
      pathname: pathname || '/',
      search: search || '',
      hash: hash || '',
      href: url,
      origin,
    };
  }

  class URLSearchParams {
    constructor(search, onChange) {
      this._entries = String(search || '')
        .replace(/^\?/, '')
        .split('&')
        .filter(Boolean)
        .map((part) => {
          const separator = part.indexOf('=');
          return separator < 0 ? [part, ''] : [part.slice(0, separator), part.slice(separator + 1)];
        });
      this._onChange = onChange;
    }

    delete(name) {
      this._entries = this._entries.filter(([key]) => decodeURIComponent(key) !== name);
      this._onChange?.(this.toString());
    }

    toString() {
      return this._entries.map(([key, value]) => (value ? `${key}=${value}` : key)).join('&');
    }
  }

  class URL {
    constructor(url, base) {
      this._set(parseUrl(url, base));
    }

    _set(parsed) {
      Object.assign(this, parsed);
      this.searchParams = new URLSearchParams(this.search, (search) => {
        this.search = search ? `?${search}` : '';
        this.href = `${this.origin}${this.pathname}${this.search}${this.hash}`;
      });
    }

    toString() {
      return this.href;
    }
  }

  globalThis.URL = URL;
  globalThis.URLSearchParams = URLSearchParams;

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
      return currentUrl.origin;
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
