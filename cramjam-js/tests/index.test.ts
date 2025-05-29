import {expect, test} from 'vitest';
import {Compress, Decompress} from '../pkg/cramjam';

const decoder = new TextDecoder();
const encoder = new TextEncoder();

test.each([
  ['brotli', Compress.brotli, Decompress.brotli],
  ['snappy', Compress.snappy, Decompress.snappy],
  ['lz4', Compress.lz4, Decompress.lz4],
])('simple round trip: %s', (_variant, compress, decompress) => {
  const str = 'hello, world';
  const encoded = encoder.encode(str);

  const compressed = compress(encoded);
  const decompressed = decompress(compressed);

  const decoded = decoder.decode(decompressed);
  expect(decoded).toBe(str);
});
