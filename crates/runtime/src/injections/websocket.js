/**
 * WebSocket polyfill for QuickJS environment
 * Purpose: Make webpack-dev-server HMR work
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
      this.readyState = WebSocket.CONNECTING;
      this.binaryType = 'arraybuffer'; // Default

      this.onopen = null;
      this.onmessage = null;
      this.onerror = null;
      this.onclose = null;

      this._id = globalThis.__moyu_ws_connect(url);
      instances.set(this._id, new WeakRef(this));
      registry.register(this, this._id);
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
        if (typeof ws.onopen === 'function') {
          ws.onopen({ type: 'open' });
        }
        break;

      case 'message':
        if (typeof ws.onmessage === 'function') {
          const data = args[0];
          ws.onmessage({ type: 'message', data: data });
        }
        break;

      case 'error':
        if (typeof ws.onerror === 'function') {
          ws.onerror({ type: 'error' });
        }
        break;

      case 'close':
        ws.readyState = WebSocket.CLOSED;
        instances.delete(id);
        if (typeof ws.onclose === 'function') {
          const [code, reason] = args;
          ws.onclose({
            type: 'close',
            wasClean: code === 1000,
            code: code,
            reason: reason,
          });
        }
        break;
    }
  };

  // Inject into global scope
  globalThis.WebSocket = WebSocket;

  if (typeof window !== 'undefined') {
    window.WebSocket = WebSocket;
  }
})();
