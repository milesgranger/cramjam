An extension to cramjam for LZO

Follows the same API, but is not distributed
with cramjam due to LZO having [GPL-2.0 license](LICENSE) which is not
compatible with cramjam's [MIT license](../LICENSE).

Therefore, should you choose to use this, you're then subject to the 
contraints of GPL-2.0

---
```bash
pip install cramjam
pip install cramjam-lzo
```

```python
# LZO will then be a submodule to cramjam
from cramjam import lzo
```
