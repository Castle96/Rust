.PHONY: build run docker-build docker-run k8s-deploy k8s-delete clean

build:
	cargo build --release

run:
	cargo run --release

docker-build:
	docker build -t santa-tracker:latest .

docker-run:
	docker run -it --rm santa-tracker:latest

# For kind clusters
kind-load:
	kind load docker-image santa-tracker:latest

k8s-deploy:
	kubectl apply -f k8s/deployment.yaml

k8s-delete:
	kubectl delete -f k8s/deployment.yaml

k8s-logs:
	kubectl logs -f -n santa-tracker deployment/santa-tracker

k8s-attach:
	kubectl attach -it -n santa-tracker deployment/santa-tracker

clean:
	cargo clean
	docker rmi santa-tracker:latest || true
