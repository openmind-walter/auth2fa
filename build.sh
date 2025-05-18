IMAGE_NAME=$1
if [[ -z "$1" ]]; then
  IMAGE_NAME=$(basename "$PWD")
fi

cp Dockerfile ..
cd ..
docker build . -t ${IMAGE_NAME}
