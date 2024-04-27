import torch
from typing import Optional

from vision_stream._lib.cuda import CudaRtspSource as CudaRtspSourceRs
from vision_stream._lib.cuda import CudaImage as _CudaImage


class CudaImage:
    def __init__(self, img_rs: _CudaImage) -> None:
        self._img = img_rs

    def to_tensor(self) -> torch.Tensor:
        tensor = torch.empty(
            self._img.shape,
            dtype=torch.uint8,
            device=f"cuda:{self._img.device}",
        )

        self._img.copy_to(tensor.data_ptr())

        return tensor


class CudaRtspSource:
    def __init__(
        self,
        uri: str,
        username: Optional[str] = None,
        password: Optional[str] = None,
    ) -> None:
        self._source = CudaRtspSourceRs(
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
