from typing import Optional

from vision_stream._lib.deepstream import NvRtspSource as NvRtspSourceRs
from vision_stream.cuda import CudaImage


class NvRtspSource:
    def __init__(
        self,
        uri: str,
    ) -> None:
        self._source = NvRtspSourceRs(uri)

    def read(self) -> Optional[CudaImage]:
        img_rs = self._source.read()
        if img_rs is None:
            return None

        return CudaImage(img_rs)
