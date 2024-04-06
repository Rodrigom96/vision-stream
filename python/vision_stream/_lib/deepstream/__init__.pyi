from typing import Optional


class NvImage:
    @property
    def width() -> int: ...

    @property
    def height() -> int: ...

    @property
    def channels() -> int: ...

    @property
    def data_ptr() -> int: ...


class NvRtspSource:
    def __init__(
        self,
        uri: str,
    ) -> None: ...

    def read(self) -> Optional[NvImage]: ...
