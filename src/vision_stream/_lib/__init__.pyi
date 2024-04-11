import numpy as np
from typing import Optional


class Image:
    def to_numpy(self) -> np.ndarray: ...


class RtspSource:
    def __init__(
        self,
        uri: str,
    ) -> None: ...

    def read(self) -> Optional[Image]: ...

    def is_reconnecting(self) -> bool: ...
