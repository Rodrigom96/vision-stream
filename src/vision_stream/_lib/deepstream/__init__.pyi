from typing import Optional

from vision_stream._lib.cuda import CudaImage


class NvRtspSource:
    def __init__(
        self,
        uri: str,
        username: Optional[str] = None,
        password: Optional[str] = None,
    ) -> None: ...

    def read(self) -> Optional[CudaImage]: ...

    def is_reconnecting(self) -> bool: ...
