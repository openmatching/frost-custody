# CONTACT

- Tel: +86-18615462063
- Email: [mingyang91@qq.com](mailto:mingyang91@qq.com)
- WeChat: mingyang91

# PERSONAL INFORMATION

- Ming Yang/Male/1991
- Working years: 12
- Github: https://github.com/mingyang91
- Zhihu: https://www.zhihu.com/people/mingyang91
- Blob: https://famer.me/
- Expected positions: Senior Developer
- Expected city: Shanghai (remote open)
- School: University of East London - BSc (Hons) Computer Science

# INTRODUCTION

- Highly proficient in Kotlin, Scala, Rust and TypeScript, and have a good level of skill in Go.
- Highly proficient in DevOps, including Kubernetes and AWS.
- Highly proficient in Reactive Systems, Asynchronous and Parallel Programming, and more. I am always eager to learn new, modern programming languages and concepts, and am able to develop professional projects using multiple languages.
- Have extensive experience in building cloud native systems.
- Have participated in a variety of projects and am currently actively coding.
- Community sharing:
    - "ZIO Introduction" at [Scala Meetup in September 2020](https://www.bilibili.com/video/BV1Qa4y1L7dj).
    - "Utilizing context receivers and suspend to create a modern backend service using **idiomatic** Kotlin" at [**KotlinConf Global 2024 Shanghai**](https://www.bilibili.com/video/BV1hxbNeaEJY)
- Have solid basic knowledge in Mathematics, Computer Science, Backend, Frontend.
- In recent years, my interests have included programming language theory, distributed systems, high-performance native software, functional programming, and mathematics.

# WORK EXPERIENCE

## BiHelix (**Apr. 2024** ~ Present)

As a senior Rust developer and architect, I joined this Bitcoin ecosystem company. My main responsibilities include upgrading the centralized wallet infrastructure architecture, code white-box review, gateway system development, and research on RGB protocol and Lightning Network.

### PoW Verification WASM Gateway Application

During the launch phase of new projects and AirDrops, applications often need to handle malicious attacks with traffic hundreds of thousands of times higher than normal, with typically 99.99% of traffic coming from various hacker teams, farming operations, and competitors. As a gateway, we cannot simply cut off all traffic - we need to identify and allow potential legitimate user requests while attempting to block brute force scripts from farming operations. Therefore, I designed a WASM firewall application running in the gateway that can implement tiered rate limiting based on IP addresses or address ranges, defining different rate limiting strategies for each interface route.

When an IP range generates too many requests, the gateway firewall begins requiring PoW verification for requests, with frontend and mobile SDK using built-in PoW computation functions to calculate valid nonces. When brute force attacks persist, the PoW difficulty gradually increases until the attack becomes unprofitable for farming operations (for example, difficulty increasing until each nonce calculation requires 10 seconds of CPU time).

Requests that fail verification are directly rejected by the gateway, while verified PoW requests increment the IP address request count. When farming operations and normal users share the same proxy route and trigger PoW rate limiting, regular users' normal operations only require 1-10 seconds of CPU time for PoW verification, avoiding complete blockage of normal user access.

### Centralized Wallet

This project serves as the fund accounting and support platform for the trading market. Before I joined the team, there were numerous security vulnerabilities and code logic errors, and some adverse financial incidents occurred after the product launch.

I was called upon during the crisis to patch security vulnerabilities left by colleagues. Afterwards, at the boss's request, I took over to rewrite the entire centralized wallet to ensure the security of deposit and withdrawal processes. The redesigned centralized wallet adopts an account design based on the ledger model, completely eliminating all financial and application security vulnerabilities.

Additionally, I integrated distributed tracing and metrics tools like Jaeger and Prometheus into the rewritten project, significantly reducing the time cycle for bug discovery and fixes, and enhancing the program's observability.

## **Vitongue - Tonomous (Jul. 2023 ~ Apr. 2024)**

Vitongue is an agency that represents Tonomous employees in the China region. I joined as a System Architect I report directly to the tech leader at KSA. My responsibilities include expanding and adding detail to high-level abstract designs, as well as implementing them.

### Smart Surveillance

This application uses a drone to monitor the environment, capture images, and identify hazards like fire, pavement cracks, and garbage. Upon detection, the system alerts the administration.

As the system architect, my duties include designing VPC, deploying Kubernetes, integrating computer vision algorithms, and coding. The project, developed entirely in Kotlin, was led by me.

I utilized tools such as context receivers, Exposed, Ktor, ArrowKt, kotlinx.serialization, coroutines, among others, and adhered to the idiomatic Kotlin coding style.

### S**imultaneous Interpretation (for HR Train)**

This application aims to reduce language barriers. Safety is the most important aspect in building construction. Helping laborers recognize the risk factors and stay away from them is the goal of the training. Today, guest laborers are commonly employed in any construction site, including Neom city. Laborers' speech in their languages is different from that of safety trainers. Therefore, the simultaneous interpretation function is a critical part of the train system.

I was tasked with developing the voice-to-voice translator module, which allows each trainer to speak in their language while laborers listen in another language. It involves three steps: transcription, translation, and text-to-speech. I refined each step as follows:

- Transcription only works once for each speaker and is broadcasted for translation.
- Translation only works once for each target language and is broadcasted for synthesis.
- Synthesis only works once for each target voice feature (male/female, younger/older).

This significantly reduces redundant inference processes, lowers cloud billing, and avoids voice desynchrony theoretically.

Additionally, I developed a lip-sync solution using three.js in the browser. A human 3D model will perform lip-sync when audio is playing.

## [**coScene](https://coscene.cn/)(Feb. 2023 ~ Jul. 2023)**

coScene is a multimodal scene data platform. It provides a complete toolchain for robotic R&D and operational needs. Taking advantage of coScene's core engines, customers will be able to efficiently track, store, transform, utilize, and share scene data on a large scale.

I joined this enterprise as an architect. During my tenure, I proposed several great designs, some of which were featured in my blog. The following designs were featured:

- [**Improving the Display of Data Lists: Designing a Real-Time Event Subscription Architecture**](https://famer.me/2023/04/15/Improving-the-Display-of-Data-Lists-Designing-a-Real-Time-Event-Subscription-Architecture/)
- [**Implement the file tree in PostgreSQL using ltree**](https://famer.me/2023/03/19/Implement-the-file%E2%80%93tree-in-PostgreSQL-using-ltree/)
- [**Implementing Version Management of File Trees in PostgreSQL**](https://famer.me/2023/03/19/Implementing-Version-Management-of-File-Trees-in-PostgreSQL/)

As a senior engineer, I always conduct research on cutting-edge technology and summarize my findings in documents.

## **Shanghai Rangchuan Information Technology Co , Ltd** (Apr. 2016 ~ Jan. **2023**)

### K/V Storage(Scala)

Using the Akka framework, I developed a low-latency, high-throughput K/V document storage system with time travel capability. The system allows data to be rolled back to any point in the past, or for the full history of data changes to be browsed.

Project performance:

- Distribution and high availability: This project can be deployed across more than 100 nodes for high-throughput if desired, and can handle a distributed storage of more than 10 billion K/V pairs.
- High-throughput and low-latency: Deploying three small nodes can achieve more than 1k QPS, with data retrieve and exception path latencies that are very close to memory access speeds.
- Event-Sourcing: The system's data can be recovered to any point in the past, and the audit log can be kept permanently.
- Convenient querying: The system has a GraphQL read-side implementation for easy querying.

Technical highlights:

- It follows the CQRS/ES design pattern fully.
- It has an interpreter that generates GraphQL Schema and Definition from Swagger (OpenAPI).
- It has integrated distributed tracing with Open Telemetry, which can automatically sample and report spans across akka-remote / kafka / knative-eventing / http + grpc, etc.
- It has excellent observability, with many metrics exposed to Prometheus.

### Backend(Scala)

Migration backend from NodeJS to the Scala Playframework and fully adopted pure functional programming with pure asynchronous behavior, type-oriented modeling using Algebraic Data Types, resulting in the elimination of `NullPointException`.

Team members: 6

Project performance:

- Currently, the system is providing real-time services to almost 3,000 tenants.
- When the system is handling 100qps, only 0.5 core is utilized.

Technical highlights:

- Replacement of the low-level asynchronous mechanics from Future to ZIO in 3 months, which resulted in the CPU usage dropping from 10 cores to less than 0.5 core during peak periods. After replacing the mechanics, I shared this experience with the Scala community.
- Data structure mapping and JSON encoding/decoding based on [`Circe`](http://circe.io/), [`Shapeless`](https://github.com/milessabin/shapeless), which involved applying Algebraic Data Types in a real-world setting. This advanced the automatic generation of codecs to compile time. Compared to common `Jackson`/`fastjson` solutions, this approach does not rely on reflection, and many bugs and vulnerabilities are blocked during compilation
- Multi-Tenancy isolation system design
- Webhook mechanism: When a business object is changed, this service actively pushes event data to the Webhook address set by the customer, records the response result from the customer's server, and automatically retries if necessary to ensure that events are not lost due to the customer's server being down or experiencing a failure.
- Resource limitation and billing
- Massively applied Reactive-Stream technology

### Infrastructure

Built and maintained a Kubernetes cluster from version 1.6 across the AWS VPC and local infrastructure, as well as the software and hardware platforms. This infrastructure supports all of the company's business and machine learning needs, including computing, storage, scheduling, monitoring, and alerting.

### River(Scala)

The Change Data Capture middleware is designed to parse PostgreSQL's transaction write-ahead log (WAL) and then write it to ElasticSearch / Kafka for downstream data service analysis and consumption.

Team members: 3

Technical background: Based on the principle of database logging, robust middleware is built using reactive streaming technologies such as [Akka Stream](notion://www.notion.so/(%3Chttps://doc.akka.io/docs/akka/current/stream/index.html%3E)).

Technical highlights:

- Parser combinator: [`parser combinator`](https://github.com/scala/scala-parser-combinators) is used to parse logical log.
- Reactive Stream: Using akka stream, stream log processing can achieve backpressure. After several generations of evolution, it has strong error self-recovery capabilities.

### Command line tool software

- Unzip
    
    Parallel multi-format decompression command-line tool. It makes full use of multi-core processors and the read/write performance of NVMe hard disks, therefore nested compression can be fast processed for hundreds of thousands of TB-level compressed package.
    
    And through application of the binary distribution version with GraalVM Native Image technology, it can start within microsecond in the cloud native platform and reduce the resource usage.
    

## Neusoft Carrier (Sep. 2014 ~ Mar. 2016)

I lead a team of three front-end developers in doing full-stack development using NodeJS.

## Qingdao Sun Valley Information Technology Service Co., Ltd. and Haier Software (Feb. 2012 ~ Aug. 2014)

Plain work

# LEARN

- Combinatorics [Certificate](https://v1-www.xuetangx.com/download_credential/DaVObVx0vXL.pdf)
- Structure and Interpretation of Computer Programs，"SICP"
- Operating system [learning](https://www.xuetangx.com/course/thu08091002729/5883981?channel=learn_title)

# SKILL LIST

The following are the skills I use proficiently.

- Backend：NodeJS / Akka / Next.JS / Scala / Prisma
- Language：Scala / Rust / Kotlin / TypeScript / Java
- Framework: Akka Stream / RxJS / React
- Tools: Cats / GraalVM / gRPC / ZIO
- Database：PostgreSQL / Cassandra / Redis / ElasticSearch
- DevOps：AWS / Kubernetes / Telemetry / Prometheus / Kafka / Serverless / Istio