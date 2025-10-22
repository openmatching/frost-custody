.PHONY: help build up-multisig up-frost up-all down logs clean clippy test

help:
	@echo "FROST Custody - Bitcoin Threshold Signing"
	@echo ""
	@echo "Available commands:"
	@echo "  make build        Build Docker image"
	@echo "  make up-multisig  Run traditional multisig (ports 3000-3002)"
	@echo "  make up-frost     Run FROST aggregator + signers (port 5000)"
	@echo "  make up-all       Run both multisig and FROST"
	@echo "  make down         Stop all services"
	@echo "  make logs         View logs"
	@echo "  make clippy       Run clippy linter (strict)"
	@echo "  make test         Run all tests"
	@echo "  make clean        Stop and remove everything"
	@echo ""
	@echo "Quick start:"
	@echo "  make build && make up-frost"

build:
	@echo "Building frost-custody image..."
	docker-compose build

up-multisig:
	@echo "Starting traditional multisig nodes (ports 3000-3002)..."
	docker-compose up -d multisig-node0 multisig-node1 multisig-node2

up-frost:
	@echo "Starting FROST aggregator + signers (port 5000)..."
	docker-compose up -d frost-node0 frost-node1 frost-node2 frost-aggregator

up-all:
	@echo "Starting all services..."
	docker-compose up -d

down:
	@echo "Stopping all services..."
	docker-compose down

logs:
	docker-compose logs -f

logs-frost:
	docker-compose logs -f frost-aggregator

logs-multisig:
	docker-compose logs -f multisig-node0

clean:
	@echo "Stopping and removing all containers, networks..."
	docker-compose down -v
	@echo "Removing image..."
	docker rmi frost-custody:latest 2>/dev/null || true

test-multisig:
	@echo "Testing multisig API..."
	curl -s 'http://127.0.0.1:3000/health' | jq .

test-frost:
	@echo "Testing FROST aggregator API..."
	curl -s 'http://127.0.0.1:6000/health' | jq .

clippy:
	@echo "Running clippy on workspace (warnings as errors)..."
	cargo clippy --workspace --all-targets -- -D warnings

test:
	@echo "Running all tests..."
	cargo test --workspace

