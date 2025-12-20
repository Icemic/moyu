// Fetch API Test
console.log('Starting Fetch test...');

async function runTest() {
  try {
    // Test 1: Fetch JSON (using a public API or local server if available)
    // For HMR testing, we usually fetch from localhost
    console.log('Fetching from https://httpbin.org/get...');
    const response = await fetch('https://httpbin.org/get');
    
    console.log('Status:', response.status);
    console.log('OK:', response.ok);
    
    const data = await response.json();
    console.log('Response JSON keys:', Object.keys(data));
    
    if (response.ok && data.url) {
      console.log('✓ Fetch JSON test passed');
    } else {
      console.log('✗ Fetch JSON test failed');
    }

    // Test 2: Fetch Text
    console.log('\nFetching text from https://httpbin.org/robots.txt...');
    const textRes = await fetch('https://httpbin.org/robots.txt');
    const text = await textRes.text();
    console.log('Text content length:', text.length);
    if (text.includes('User-agent')) {
      console.log('✓ Fetch text test passed');
    } else {
      console.log('✗ Fetch text test failed');
    }

  } catch (e) {
    console.error('Fetch test error:', e);
  }
}

runTest();
