
## De/Compression algorithms to Rust's `libcramjam` using WASM.


### Use:

```typescript

import {Compress, Decompress} from 'cramjam';

const decoder = new TextDecoder();
const encoder = new TextEncoder();

const str = 'hello, world';
const encoded = encoder.encode(str);

const compressed = Compress.brotli(encoded);
const decompressed = Decompress.brotli(compressed);

const decoded = decoder.decode(decompressed);

```


### Supported algorithms:

- `De/Compress.brotli`
- `De/Compress.snappy`
- `De/Compress.lz4`
- ...more to come...
