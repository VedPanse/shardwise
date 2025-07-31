# Shardwise

**Shardwise** is a horizontally scalable, microservice-based system for computing backpropagation gradients in a distributed fashion. Built as a capstone project for mastering real-world system design principles, Shardwise is not a product — it's an **engineering statement.**

## 🚀 What It Does

Shardwise accepts model parameters and forward-pass inputs, splits the gradient computation into discrete tasks, and dispatches them across a fleet of compute microservices. Results are asynchronously collected, cached, and streamed back to the requester.

## 🎯 Goals

* Showcase deep understanding of system design fundamentals
* Build an elastic, fault-tolerant gradient compute system
* Demonstrate DevOps and infra engineering skills (Docker, Redis, Kafka, Nginx)
* Explore CAP tradeoffs through real-world design decisions

---

## 🧱 Architecture Overview

### Components:

| Service               | Description                                                |
| --------------------- | ---------------------------------------------------------- |
| **API Gateway**       | Accepts HTTP requests and routes to backend services       |
| **Shard Scheduler**   | Breaks gradient jobs into tasks and pushes to Kafka        |
| **Worker Pool**       | Stateless microservices that compute partial gradients     |
| **Result Aggregator** | Pulls partial results, combines gradients, and stores them |
| **Cache Layer**       | Redis-based, caches frequent inputs/results                |
| **Result Store**      | PostgreSQL DB storing metadata and computed gradients      |
| **Monitoring**        | Prometheus + Grafana for system health and metrics         |

### Message Flow:

1. Client sends POST to `/api/backprop`
2. API Gateway routes to `Shard Scheduler`
3. Scheduler shards job and pushes messages to Kafka
4. Worker Pool subscribes to Kafka topics, computes gradients
5. Results are aggregated and stored
6. User fetches result via `/api/result/:id` or receives streaming update

---

## 🛠️ Tech Stack

| Layer          | Tech Used                 |
| -------------- | ------------------------- |
| API & Services | Python (FastAPI), Node.js |
| Queueing       | Kafka or RabbitMQ         |
| Caching        | Redis                     |
| DB             | PostgreSQL                |
| Orchestration  | Docker + Docker Compose   |
| Load Balancing | Nginx                     |
| Monitoring     | Prometheus, Grafana       |

---

## ⚖️ Devverse Design Mappings

| Devverse Topic            | Implementation in Shardwise                       |
| ------------------------- | ------------------------------------------------- |
| Monolith vs Microservices | Broken into 6+ clear services                     |
| Load Balancing & Scaling  | Nginx + horizontally-scalable workers             |
| Caching                   | Redis for input/result caching                    |
| Message Queues            | Kafka used for async compute pipelines            |
| Scalable API              | REST endpoints documented via Swagger/OpenAPI     |
| CAP Theorem               | Prioritize A for ingestion, C for result delivery |
| Nginx                     | Used as reverse proxy + load balancer             |

---

## 📦 API Endpoints

### POST `/api/backprop`

**Description:** Submit a new gradient job
**Body:**

```json
{
  "model": "MLP",
  "inputs": [[1.0, 2.0]],
  "weights": [[0.5, -0.2], [1.3, 0.7]]
}
```

**Response:**

```json
{ "job_id": "abc123" }
```

### GET `/api/result/:job_id`

Returns computed gradients or processing status.

### GET `/api/status/:job_id`

Returns status: `queued`, `processing`, `done`, `failed`

---

## 📈 Scaling Strategy

* Stateless services = trivially scalable
* Horizontal pod autoscaling on Worker Pool
* Kafka consumers scale with partitions
* Redis reduces compute for repeated queries

---

## 📊 Monitoring & Observability

* **Prometheus** tracks job latency, worker load
* **Grafana** dashboards show system health
* **AlertManager** (optional) for real-time ops alerts

---

## 🧪 Testing & Simulation

* Includes test harness to simulate large job loads
* Metrics tracking for throughput, failure rate
* Can integrate mock models to simulate training pipelines

---

## 📄 License & Intent

This project is educational and experimental. Shardwise is not meant for production use, but as a **resume-level showcase of distributed system design**.

---

## 👨‍🚀 Author

**Ved Panse**
[vedpanse.com](https://vedpanse.com)
UC San Diego | B.S. Math-CS & Data Science
