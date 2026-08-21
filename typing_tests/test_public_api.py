from typing import assert_type

import cramjam
from cramjam import Buffer

data: bytearray
output: bytearray

if False:
    assert_type(cramjam.snappy.compress(data), Buffer)
    assert_type(cramjam.snappy.compress_into(data, output), int)
    assert_type(cramjam.snappy.decompress_into(data, bytes(256)), int)
    assert_type(cramjam.lz4.compress_block_into(data, output, compression=9), int)

    decompressor = cramjam.zstd.Decompressor()
    assert_type(decompressor.decompress(data), int)
    assert_type(decompressor.flush(), Buffer)
    assert_type(decompressor.finish(), Buffer)

    options = cramjam.xz.Options().set_mode(cramjam.xz.Mode.Normal)
    filters = cramjam.xz.FilterChain()
    filters.append_filter(cramjam.xz.FilterChainItem(cramjam.xz.Filter.Lzma2, options))
    assert_type(
        cramjam.xz.compress_into(
            data,
            output,
            format=cramjam.xz.Format.XZ,
            check=cramjam.xz.Check.NONE,
            filters=filters,
        ),
        int,
    )

    assert_type(cramjam.experimental.blosc2.compress_chunk(data), Buffer)
    assert_type(cramjam.experimental.blosc2.max_compressed_len(len(data)), int)
    assert_type(cramjam.experimental.igzip.compress(data), Buffer)
