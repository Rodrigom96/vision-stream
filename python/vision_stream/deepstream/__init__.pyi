import torch
from typing import Optional


class NvRtspSource:
    def __init__(
        self,
        uri: str,
    ) -> None: ...

    def read(self) -> Optional[torch.Tensor]: ...
