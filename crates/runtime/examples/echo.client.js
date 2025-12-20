// WebSocket Echo Client Test
// This will be evaluated in Moyu runtime

(() => {
  console.log('Starting WebSocket test...');

  const WS_URL = 'ws://localhost:9001';
  const ws = new WebSocket(WS_URL);

  let testsPassed = 0;
  let testsFailed = 0;

  function logTest(name, passed, message) {
    if (passed) {
      testsPassed++;
      console.log(`✓ ${name}`);
    } else {
      testsFailed++;
      console.log(`✗ ${name}: ${message}`);
    }
  }

  // Test 1: Connection
  ws.onopen = function (event) {
    logTest('Connection established', true);
    console.log('WebSocket connected to:', WS_URL);
    console.log('ReadyState:', ws.readyState, '(expected: 1)');

    // Test 2: Send text message
    const textMessage = 'Hello, Echo Server!';
    console.log('\nSending text message:', textMessage);
    ws.send(textMessage);
  };

  // Message handler
  let receivedMessages = 0;
  ws.onmessage = function (event) {
    receivedMessages++;
    console.log(`\nReceived message #${receivedMessages}:`, event.data);

    if (receivedMessages === 1) {
      // First message is the welcome message
      logTest(
        'Received welcome message',
        typeof event.data === 'string' && event.data.includes('Welcome'),
        `Got: ${event.data}`,
      );
    } else if (receivedMessages === 2) {
      // Echo of our text message
      logTest(
        'Text echo received',
        event.data === 'Hello, Echo Server!',
        `Expected "Hello, Echo Server!", got "${event.data}"`,
      );

      // Test 3: Send binary message
      console.log('\nSending binary message...');
      const binaryData = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
      ws.send(binaryData.buffer);
    } else if (receivedMessages === 3) {
      // Echo of binary message
      const isBinary = event.data instanceof ArrayBuffer || (typeof Blob !== 'undefined' && event.data instanceof Blob);
      logTest('Binary echo received', isBinary, `Expected ArrayBuffer or Blob, got ${typeof event.data}`);

      if (isBinary) {
        // Handle both ArrayBuffer and Blob
        if (event.data instanceof ArrayBuffer) {
          const bytes = new Uint8Array(event.data);
          const expected = [72, 101, 108, 108, 111];
          const matches = bytes.length === expected.length && bytes.every((val, i) => val === expected[i]);
          logTest('Binary data matches', matches, `Expected [${expected}], got [${Array.from(bytes)}]`);

          // Test 4: Close connection
          console.log('\nClosing connection...');
          ws.close(1000, 'Test completed');
        } else if (typeof Blob !== 'undefined' && event.data instanceof Blob) {
          // In Node.js, binary data comes as Blob
          event.data.arrayBuffer().then((buffer) => {
            const bytes = new Uint8Array(buffer);
            const expected = [72, 101, 108, 108, 111];
            const matches = bytes.length === expected.length && bytes.every((val, i) => val === expected[i]);
            logTest('Binary data matches', matches, `Expected [${expected}], got [${Array.from(bytes)}]`);

            // Test 4: Close connection
            console.log('\nClosing connection...');
            ws.close(1000, 'Test completed');
          });
        }
      } else {
        // If not binary, close anyway
        console.log('\nClosing connection...');
        ws.close(1000, 'Test completed');
      }
    }
  };

  // Error handler
  ws.onerror = function (event) {
    logTest('No errors', false, 'WebSocket error occurred');
    console.error('WebSocket error:', event);
  };

  // Close handler
  ws.onclose = function (event) {
    console.log('\nWebSocket closed');
    console.log('Code:', event.code);
    console.log('Reason:', event.reason);
    console.log('Was clean:', event.wasClean);

    logTest('Clean close', event.wasClean && event.code === 1000, `Code: ${event.code}, wasClean: ${event.wasClean}`);

    // Print summary
    console.log('\n' + '='.repeat(50));
    console.log('Test Summary:');
    console.log(`Passed: ${testsPassed}`);
    console.log(`Failed: ${testsFailed}`);
    console.log(`Total:  ${testsPassed + testsFailed}`);
    console.log('='.repeat(50));

    if (testsFailed === 0) {
      console.log('\n🎉 All tests passed!');
    } else {
      console.log('\n❌ Some tests failed!');
    }
  };

  console.log('WebSocket test initialized. Connecting to', WS_URL);
  console.log('Initial readyState:', ws.readyState, '(expected: 0 - CONNECTING)');
})();
