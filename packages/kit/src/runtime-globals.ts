export {};

declare global {
  /**
   * The **`console`** object provides access to the debugging console (e.g., the Web console in Firefox).
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console)
   */
  interface Console {
    /**
     * The **`console.debug()`** static method outputs a message to the console at the 'debug' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/debug_static)
     */
    debug(...data: any[]): void;
    /**
     * The **`console.error()`** static method outputs a message to the console at the 'error' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/error_static)
     */
    error(...data: any[]): void;
    /**
     * The **`console.info()`** static method outputs a message to the console at the 'info' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/info_static)
     */
    info(...data: any[]): void;
    /**
     * The **`console.log()`** static method outputs a message to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/log_static)
     */
    log(...data: any[]): void;
    /**
     * The **`console.trace()`** static method outputs a stack trace to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/trace_static)
     */
    trace(...data: any[]): void;
    /**
     * The **`console.warn()`** static method outputs a warning message to the console at the 'warning' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/warn_static)
     */
    warn(...data: any[]): void;
  }

  var console: Console;

  type TimerHandler = string | Function;

  function clearInterval(id: number | undefined): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Window/clearTimeout) */
  function clearTimeout(id: number | undefined): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Window/setInterval) */
  function setInterval(handler: TimerHandler, timeout?: number, ...args: any[]): number;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Window/setTimeout) */
  function setTimeout(handler: TimerHandler, timeout?: number, ...args: any[]): number;

  type DOMHighResTimeStamp = number;

  interface FrameRequestCallback {
    (time: DOMHighResTimeStamp): void;
  }

  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/DedicatedWorkerGlobalScope/cancelAnimationFrame) */
  function cancelAnimationFrame(handle: number): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/DedicatedWorkerGlobalScope/requestAnimationFrame) */
  function requestAnimationFrame(callback: FrameRequestCallback): number;
}
