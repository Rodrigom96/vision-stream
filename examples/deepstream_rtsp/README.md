Build docker
```
docker compose build
```
Allow external applications to connect to the host's X display:
```
xhost +
```
Run example
```
docker compose run --rm app python3 main.py <RTSP_PATH>
```