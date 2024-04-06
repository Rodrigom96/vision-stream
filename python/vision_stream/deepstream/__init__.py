import torch
from cuda import cuda
from typing import Optional

from .._lib.deepstream import (
    NvRtspSource as NvRtspSourceRs,
    NvImage as NvImageRs,
)


class NvImage:
    def __init__(self, img_rs: NvImageRs) -> None:
        self._img = img_rs

    def to_tensor(self) -> torch.Tensor:
        tensor = torch.empty(
            (self._img.height, self._img.width, self._img.channels),
            dtype=torch.uint8,
            device="cuda",
        )

        size = self._img.height*self._img.width*self._img.channels
        cuda.cuMemcpyDtoD(tensor.data_ptr(), self._img.data_ptr, size)

        return tensor


class NvRtspSource:
    def __init__(
        self,
        uri: str,
    ) -> None:
        self._source = NvRtspSourceRs(uri)

    def read(self) -> Optional[NvImage]:
        img_rs = self._source.read()
        if img_rs is None:
            return None

        return NvImage(img_rs)
