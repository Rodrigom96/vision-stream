from typing import Optional

from vision_stream._lib.deepstream import NvRtspSource as NvRtspSourceRs
from vision_stream.cuda import CudaImage


class NvRtspSource:
    def __init__(
        self,
        uri: str,
        username: Optional[str] = None,
        password: Optional[str] = None,
    ) -> None:
        self._source = NvRtspSourceRs(
            uri,
            username=username,
            password=password,
        )

    def read(self) -> Optional[CudaImage]:
        img_rs = self._source.read()
        if img_rs is None:
            return None

        return CudaImage(img_rs)

    def is_reconnecting(self) -> bool:
        return self._source.is_reconnecting()
