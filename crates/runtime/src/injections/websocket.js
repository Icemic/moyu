/**
 * WebSocket polyfill for QuickJS environment
 * Provides the browser-compatible event API required by development server HMR
 */

(function () {
  const instances = new Map(); // id -> WeakRef<WebSocket>
  const registry = new FinalizationRegistry((id) => {
    // When the JS object is GC'd, ensure the underlying connection is closed
    try {
      globalThis.__moyu_ws_close(id, 1001, 'Garbage Collected');
    } catch (e) {
      // Ignore errors during GC cleanup
    }
  });

  class WebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;

    constructor(url, protocols) {
      this.url = url;
      this.protocol = '';
      this.readyState = WebSocket.CONNECTING;
      this.binaryType = 'arraybuffer'; // Default
      this.CONNECTING = WebSocket.CONNECTING;
      this.OPEN = WebSocket.OPEN;
      this.CLOSING = WebSocket.CLOSING;
      this.CLOSED = WebSocket.CLOSED;

      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.onclose = null;
      this._listeners = new Map();

      const protocolList = Array.isArray(protocols) ? protocols : protocols ? [protocols] : [];
      this._id = globalThis.__moyu_ws_connect(url, protocolList.join(', '));
      instances.set(this._id, new WeakRef(this));
      registry.register(this, this._id);
    }

    addEventListener(type, listener, options) {
      if (typeof listener !== 'function') return;

      const listeners = this._listeners.get(type) || [];
      if (!listeners.some((entry) => entry.listener === listener)) {
        listeners.push({ listener, once: options === true || options?.once === true });
        this._listeners.set(type, listeners);
      }
    }

    removeEventListener(type, listener) {
      const listeners = this._listeners.get(type);
      if (!listeners) return;

      const remaining = listeners.filter((entry) => entry.listener !== listener);
      if (remaining.length > 0) {
        this._listeners.set(type, remaining);
      } else {
        this._listeners.delete(type);
      }
    }

    _dispatch(type, event) {
      const handler = this[`on${type}`];
      if (typeof handler === 'function') handler.call(this, event);

      const listeners = this._listeners.get(type);
      if (!listeners) return;

      for (const entry of [...listeners]) {
        entry.listener.call(this, event);
        if (entry.once) this.removeEventListener(type, entry.listener);
      }
    }

    send(data) {
      if (this.readyState !== WebSocket.OPEN) {
        console.error('WebSocket is not open');
        return;
      }
      try {
        globalThis.__moyu_ws_send(this._id, data);
      } catch (e) {
        console.error('WebSocket send error:', e);
      }
    }

    close(code, reason) {
      if (this.readyState === WebSocket.CLOSING || this.readyState === WebSocket.CLOSED) {
        return;
      }
      this.readyState = WebSocket.CLOSING;
      globalThis.__moyu_ws_close(this._id, code, reason);
    }
  }

  // Global dispatcher called from Rust
  globalThis.__moyu_ws_dispatch = function (id, type, ...args) {
    const ref = instances.get(id);
    const ws = ref ? ref.deref() : null;

    if (!ws) {
      // If the JS object is gone, we should probably close the connection
      // if it's not already closed.
      if (type !== 'close') {
        globalThis.__moyu_ws_close(id, 1001, "Object GC'd");
      }
      instances.delete(id);
      return;
    }

    switch (type) {
      case 'open':
        ws.readyState = WebSocket.OPEN;
        ws._dispatch('open', { type: 'open', target: ws });
        break;

      case 'message':
        ws._dispatch('message', { type: 'message', data: args[0], target: ws });
        break;

      case 'error':
        ws._dispatch('error', { type: 'error', target: ws });
        break;

      case 'close': {
        ws.readyState = WebSocket.CLOSED;
        instances.delete(id);
        const [code, reason] = args;
        ws._dispatch('close', {
          type: 'close',
          wasClean: code === 1000,
          code: code,
          reason: reason,
          target: ws,
        });
        break;
      }
    }
  };

  // Inject into global scope
  globalThis.WebSocket = WebSocket;

  if (typeof window !== 'undefined') {
    window.WebSocket = WebSocket;
  }
})();
