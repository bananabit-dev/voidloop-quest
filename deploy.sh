#!/bin/bash

# Build and deployment script for Voidloop Quest
# Usage: ./deploy.sh [build|run|push|all] [environment]

set -e

# Configuration
PROJECT_NAME="voidloop-quest"
REGISTRY=${DOCKER_REGISTRY:-""}  # Set your registry, e.g., "gcr.io/myproject"
TAG=${VERSION:-"latest"}
ENVIRONMENT=${2:-"development"}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
check_directory() {
    if [ ! -f "Cargo.toml" ]; then
        log_error "This script must be run from the voidloop-quest directory"
        exit 1
    fi
}

# Build Docker images
build_images() {
    log_info "Building Docker images..."
    
    # Build from parent directory to include bevygap
    cd ..
    
    log_info "Building client image..."
    docker build -f voidloop-quest/client/Dockerfile -t ${PROJECT_NAME}-client:${TAG} .
    
    log_info "Building server image..."
    docker build -f voidloop-quest/server/Dockerfile -t ${PROJECT_NAME}-server:${TAG} .
    
    # Optionally build lobby if it exists
    if [ -f "voidloop-quest/tools/lobby/Dockerfile" ]; then
        log_info "Building lobby image..."
        docker build -f voidloop-quest/tools/lobby/Dockerfile -t ${PROJECT_NAME}-lobby:${TAG} .
    fi
    
    cd voidloop-quest
    log_info "Build complete!"
}

# Run locally with docker-compose
run_local() {
    log_info "Starting services with docker-compose..."
    
    # Generate random secrets for development
    if [ "$ENVIRONMENT" = "development" ]; then
        export LOBBY_API_KEY=$(openssl rand -hex 32)
        export JWT_SECRET=$(openssl rand -hex 32)
        export DB_PASSWORD=$(openssl rand -hex 16)
        export GRAFANA_PASSWORD="admin"
        
        log_warn "Generated development secrets (not for production use)"
    fi
    
    docker-compose up -d
    
    log_info "Services started!"
    log_info "Client: http://localhost:8080"
    log_info "Server: UDP port 6420, WebTransport port 6421"
    # log_info "Lobby: http://localhost:3000"
    # log_info "Grafana: http://localhost:3001 (admin/${GRAFANA_PASSWORD})"
}

# Push images to registry
push_images() {
    if [ -z "$REGISTRY" ]; then
        log_error "DOCKER_REGISTRY environment variable not set"
        exit 1
    fi
    
    log_info "Pushing images to ${REGISTRY}..."
    
    # Tag images
    docker tag ${PROJECT_NAME}-client:${TAG} ${REGISTRY}/${PROJECT_NAME}-client:${TAG}
    docker tag ${PROJECT_NAME}-server:${TAG} ${REGISTRY}/${PROJECT_NAME}-server:${TAG}
    
    # Push images
    docker push ${REGISTRY}/${PROJECT_NAME}-client:${TAG}
    docker push ${REGISTRY}/${PROJECT_NAME}-server:${TAG}
    
    if [ -f "tools/lobby/Dockerfile" ]; then
        docker tag ${PROJECT_NAME}-lobby:${TAG} ${REGISTRY}/${PROJECT_NAME}-lobby:${TAG}
        docker push ${REGISTRY}/${PROJECT_NAME}-lobby:${TAG}
    fi
    
    log_info "Push complete!"
}

# Deploy to Kubernetes
deploy_k8s() {
    log_info "Deploying to Kubernetes..."
    
    # Check if kubectl is available
    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl not found. Please install kubectl."
        exit 1
    fi
    
    # Apply Kubernetes manifests
    kubectl apply -f k8s/namespace.yaml
    kubectl apply -f k8s/configmap.yaml
    kubectl apply -f k8s/secrets.yaml
    kubectl apply -f k8s/client-deployment.yaml
    kubectl apply -f k8s/server-deployment.yaml
    kubectl apply -f k8s/ingress.yaml
    
    log_info "Kubernetes deployment complete!"
}

# Clean up resources
cleanup() {
    log_info "Cleaning up..."
    
    docker-compose down -v
    docker system prune -f
    
    log_info "Cleanup complete!"
}

# Health check
health_check() {
    log_info "Performing health check..."
    
    # Check client
    if curl -f http://localhost:8080/health > /dev/null 2>&1; then
        log_info "Client health check: ✓"
    else
        log_warn "Client health check: ✗"
    fi
    
    # Check server (basic port check)
    if nc -zu localhost 6420 2>/dev/null; then
        log_info "Server UDP port check: ✓"
    else
        log_warn "Server UDP port check: ✗"
    fi
}

# Main script logic
main() {
    check_directory
    
    case "$1" in
        build)
            build_images
            ;;
        run)
            run_local
            sleep 5
            health_check
            ;;
        push)
            push_images
            ;;
        deploy)
            deploy_k8s
            ;;
        clean)
            cleanup
            ;;
        health)
            health_check
            ;;
        all)
            build_images
            run_local
            sleep 5
            health_check
            ;;
        *)
            echo "Usage: $0 {build|run|push|deploy|clean|health|all} [environment]"
            echo ""
            echo "Commands:"
            echo "  build   - Build Docker images"
            echo "  run     - Run services locally with docker-compose"
            echo "  push    - Push images to registry"
            echo "  deploy  - Deploy to Kubernetes"
            echo "  clean   - Clean up resources"
            echo "  health  - Check service health"
            echo "  all     - Build and run locally"
            echo ""
            echo "Environments: development, staging, production"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
