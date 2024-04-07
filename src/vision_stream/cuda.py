import torch

from vision_stream._lib.deepstream import CudaImage as _CudaImage


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
