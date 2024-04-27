from typing import Optional, Tuple


class CudaImage:
    @property
    def width(self) -> int: ...

    @property
    def height(self) -> int: ...

    @property
    def channels(self) -> int: ...

    @property
    def data_ptr(self) -> int: ...

    @property
    def device(self) -> int: ...

    @property
    def shape(self) -> Tuple[int, int, int]: ...

    def copy_to(self, data_ptr: int): ...


class CudaRtspSource:
    def __init__(
        self,
        uri: str,
        username: Optional[str] = None,
        password: Optional[str] = None,
    ) -> None: ...

    def read(self) -> Optional[CudaImage]: ...

    def is_reconnecting(self) -> bool: ...
