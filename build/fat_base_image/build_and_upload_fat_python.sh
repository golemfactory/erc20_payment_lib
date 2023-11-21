# login to ghcr.io
docker login ghcr.io -u ${GITHUB_USER} -p ${GITHUB_TOKEN}

# build fat base
docker build \
  --label "org.opencontainers.image.source=https://github.com/golemfactory/erc20_payment_lib" \
  --label "org.opencontainers.image.description=Feature rich but fat python image with tools" \
  --label "org.opencontainers.image.licenses=MIT" \
  -t ghcr.io/golemfactory/python_fat:3.10 \
  .

docker push ghcr.io/golemfactory/python_fat:3.10